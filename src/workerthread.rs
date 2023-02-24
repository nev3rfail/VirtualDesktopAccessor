/// Purpose of this module is to provide single threaded access to COM objects
/// from any thread.
///
use super::Result;
use crate::comobjects::ComObjects;
use crate::log::log_output;
use crate::Error;
use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::sync::Mutex;
use std::sync::RwLock;
use windows::Win32::System::Threading::GetCurrentThread;
use windows::Win32::System::Threading::SetThreadPriority;
use windows::Win32::System::Threading::THREAD_PRIORITY_TIME_CRITICAL;

type ComFn = Box<dyn Fn(&ComObjects) + Send + 'static>;

pub struct WorkerThread {
    thread: RwLock<Option<std::thread::JoinHandle<()>>>,
    sender: RwLock<Option<std::sync::mpsc::SyncSender<ComFn>>>,
}

impl WorkerThread {
    pub fn new() -> Self {
        WorkerThread {
            thread: RwLock::new(None),
            sender: RwLock::new(None),
        }
    }

    pub fn send(&self, f: ComFn) -> Result<()> {
        if let Some(sender) = self.sender.read().unwrap().as_ref() {
            // Send to existing thread
            return sender.send(Box::new(f)).map_err(|_| Error::SenderError);
        }

        // Create a new thread and send
        let (sender, receiver) = std::sync::mpsc::sync_channel::<ComFn>(10);

        self.thread
            .write()
            .unwrap()
            .replace(std::thread::spawn(move || {
                log_output("Starting worker thread");

                // Set thread priority to time critical, explorer.exe really
                // hates if your com object accessing is slow.
                unsafe { SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_TIME_CRITICAL) };

                let com = ComObjects::new();
                for f in receiver.iter() {
                    f(&com);
                }
            }));
        sender.send(Box::new(f)).map_err(|_| Error::SenderError)?;
        self.sender.write().unwrap().replace(sender);
        Ok(())
    }

    fn stop(&self) -> std::thread::Result<()> {
        {
            // Drop the sender, this ends the loop in worker thread
            self.sender.write().unwrap().take();
        }

        if let Some(thread) = self.thread.write().unwrap().take() {
            thread.join()?;
            log_output("Worker thread closed");
        }

        Ok(())
    }
}

impl Drop for WorkerThread {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

static WORKER_CHANNEL: Lazy<WorkerThread> = Lazy::new(|| {
    unsafe { atexit(atexit_stop_worker_channel) };
    WorkerThread::new()
});

extern "C" fn atexit_stop_worker_channel() {
    let _ = WORKER_CHANNEL.stop();
}

extern "C" {
    fn atexit(callback: extern "C" fn()) -> std::os::raw::c_int;
}

/// Internally the COM objects are accessed in a single thread. This function
/// stops the worker thread. I don't know why would you, but it's here.
#[doc(hidden)]
pub fn stop_desktop_com_worker() {
    let _ = WORKER_CHANNEL.stop();
}

/// This is a helper function to initialize and run COM related functions in a
/// a single thread.
///
/// Virtual Desktop COM Objects don't like to being called from different
/// threads rapidly, something goes wrong. This function ensures that all COM
/// calls are done in a single thread.
pub fn with_com_objects<F, T>(f: F) -> Result<T>
where
    F: Fn(&ComObjects) -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    // Naive implementation, but not very performant
    // return std::thread::scope(|env| {
    //     let com = ComObjects::new();
    //     f(&com)
    // });

    let (sender, receiver) = std::sync::mpsc::channel();
    WORKER_CHANNEL.send(Box::new(move |c| {
        // Retry the function up to 5 times if it gives an error
        let mut r = f(c);
        for _ in 0..5 {
            match &r {
                Err(er)
                    if er == &Error::ClassNotRegistered
                        || er == &Error::RpcServerNotAvailable
                        || er == &Error::ComObjectNotConnected
                        || er == &Error::ComAllocatedNullPtr =>
                {
                    #[cfg(debug_assertions)]
                    log_output(&format!("Retry the function after {:?}", er));

                    // Explorer.exe has mostlikely crashed, retry the function
                    c.drop_services();
                    r = f(c);
                    continue;
                }
                other => {
                    // Show the error
                    #[cfg(debug_assertions)]
                    if let Err(er) = &other {
                        log_output(&format!("with_com_objects failed with {:?}", er));
                    }

                    // Return the Result
                    break;
                }
            }
        }
        let send_result = sender.send(r);
        if let Err(e) = send_result {
            #[cfg(debug_assertions)]
            log_output(&format!("with_com_objects failed to send result {:?}", e));
        }
    }))?;

    receiver.recv().map_err(|_| Error::ReceiverError)?
}