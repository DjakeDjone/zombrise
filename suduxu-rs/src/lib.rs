use lazy_static::lazy_static;
use libloading::{Library, Symbol};
use std::ffi::{CStr, CString};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

mod ffi;
pub use ffi::{ButtonInputState, ButtonInputType, SensorDataRaw};

lazy_static! {
    static ref EVENT_CALLBACK: Mutex<Option<Box<dyn Fn(&str) + Send + Sync>>> = Mutex::new(None);
    static ref SENSOR_CALLBACK: Mutex<Option<Box<dyn Fn(&SensorDataRaw) + Send + Sync>>> =
        Mutex::new(None);
}

extern "C" fn internal_event_callback(ptr: *const libc::c_char) {
    if ptr.is_null() {
        return;
    }
    let c_str = unsafe { CStr::from_ptr(ptr) };
    if let Ok(s) = c_str.to_str() {
        let callback = EVENT_CALLBACK.lock().unwrap();
        if let Some(cb) = &*callback {
            cb(s);
        }
    }
}

extern "C" fn internal_sensor_callback(data: *mut SensorDataRaw) {
    if data.is_null() {
        return;
    }
    let callback = SENSOR_CALLBACK.lock().unwrap();
    if let Some(cb) = &*callback {
        unsafe { cb(&*data) };
    }
}

pub struct Suduxu {
    lib: Arc<Library>,
    server_thread: Mutex<Option<JoinHandle<()>>>,
}

impl Suduxu {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let lib_name = "libsuduxu.so";
        let lib = unsafe { Library::new(lib_name) }
            .or_else(|_| unsafe { Library::new("./libsuduxu.so") })
            .or_else(|_| unsafe { Library::new("../libsuduxu.so") })?;

        let lib = Arc::new(lib);

        // Register callbacks
        unsafe {
            let reg_event: Symbol<ffi::RegisterEventCallbackFn> =
                lib.get(b"register_event_callback")?;
            reg_event(internal_event_callback);

            let reg_sensor: Symbol<ffi::RegisterSensorEventCallbackFn> =
                lib.get(b"register_sensor_event_callback")?;
            reg_sensor(internal_sensor_callback);
        }

        Ok(Suduxu {
            lib,
            server_thread: Mutex::new(None),
        })
    }

    pub fn start(&self) {
        let mut thread_handle = self.server_thread.lock().unwrap();
        if thread_handle.is_some() {
            return;
        }

        let lib = self.lib.clone();
        let handle = thread::spawn(move || unsafe {
            if let Ok(func) = lib.get::<ffi::StartSuduxuFn>(b"start_suduxu") {
                func();
            }
        });
        *thread_handle = Some(handle);
    }

    pub fn stop(&self) {
        unsafe {
            if let Ok(func) = self.lib.get::<ffi::StopSuduxuFn>(b"stop_suduxu") {
                func();
            }
        }
        let mut thread_handle = self.server_thread.lock().unwrap();
        if let Some(handle) = thread_handle.take() {
            let _ = handle.join();
        }
    }

    pub fn is_running(&self) -> bool {
        unsafe {
            self.lib
                .get::<ffi::IsRunningFn>(b"is_running")
                .map(|f| f())
                .unwrap_or(false)
        }
    }

    pub fn tick(&self, delta: f32) {
        unsafe {
            if let Ok(func) = self.lib.get::<ffi::TickFn>(b"tick") {
                func(delta);
            }
        }
    }

    pub fn get_button(
        &self,
        client_id: u16,
        btn: ButtonInputType,
        state: ButtonInputState,
    ) -> bool {
        unsafe {
            self.lib
                .get::<ffi::GetButtonInStateFn>(b"get_button_in_state")
                .map(|f| f(client_id, btn, state) != 0)
                .unwrap_or(false)
        }
    }

    pub fn get_sensor_data(&self, client_id: u16) -> SensorDataRaw {
        unsafe {
            self.lib
                .get::<ffi::GetSensorDataFn>(b"get_sensor_data")
                .map(|f| f(client_id))
                .unwrap_or([0; 64])
        }
    }

    pub fn disconnect_client(&self, id: u16) {
        unsafe {
            if let Ok(f) = self
                .lib
                .get::<ffi::DisconnectClientFn>(b"disconnect_client")
            {
                f(id);
            }
        }
    }

    pub fn disconnect_all(&self) {
        unsafe {
            if let Ok(f) = self.lib.get::<ffi::DisconnectAllFn>(b"disconnect_all") {
                f();
            }
        }
    }

    pub fn find_all_clients(&self) -> String {
        unsafe {
            let func = self.lib.get::<ffi::FindAllClientsFn>(b"find_all_clients");
            if let Ok(f) = func {
                let ptr = f();
                if ptr.is_null() {
                    return String::new();
                }
                let c_str = CStr::from_ptr(ptr);
                let s = c_str.to_string_lossy().into_owned();

                // Free using library's free
                if let Ok(free_func) = self.lib.get::<ffi::FreeFn>(b"free") {
                    free_func(ptr as *mut _);
                }
                return s;
            }
        }
        String::new()
    }

    pub fn find_client_by_id(&self, id: u16) -> String {
        unsafe {
            let func = self.lib.get::<ffi::FindClientByIdFn>(b"find_client_by_id");
            if let Ok(f) = func {
                let ptr = f(id);
                if ptr.is_null() {
                    return String::new();
                }
                let c_str = CStr::from_ptr(ptr);
                let s = c_str.to_string_lossy().into_owned();
                if let Ok(free_func) = self.lib.get::<ffi::FreeFn>(b"free") {
                    free_func(ptr as *mut _);
                }
                return s;
            }
        }
        String::new()
    }

    pub fn broadcast_tcp(&self, message: &str) {
        let c_str = CString::new(message).expect("CString::new failed");
        unsafe {
            if let Ok(f) = self.lib.get::<ffi::BroadcastTcpFn>(b"broadcast_tcp") {
                f(c_str.as_ptr());
            }
        }
    }

    pub fn send_to_client(&self, id: u16, message: &str) {
        let c_str = CString::new(message).expect("CString::new failed");
        unsafe {
            if let Ok(f) = self.lib.get::<ffi::SendToClientFn>(b"send_to_client") {
                f(id, c_str.as_ptr());
            }
        }
    }

    pub fn set_event_callback<F>(&self, callback: F)
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        let mut cb = EVENT_CALLBACK.lock().unwrap();
        *cb = Some(Box::new(callback));
    }

    pub fn set_sensor_callback<F>(&self, callback: F)
    where
        F: Fn(&SensorDataRaw) + Send + Sync + 'static,
    {
        let mut cb = SENSOR_CALLBACK.lock().unwrap();
        *cb = Some(Box::new(callback));
    }
}

impl Drop for Suduxu {
    fn drop(&mut self) {
        self.stop();
    }
}
