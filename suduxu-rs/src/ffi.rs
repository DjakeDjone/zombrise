use libc::{c_char, c_float, c_ushort, c_void};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonInputType {
    Up = 0,
    Right = 1,
    Down = 2,
    Left = 3,
    A = 4,
    B = 5,
    X = 6,
    Y = 7,
    Minus = 8,
    Plus = 9,
    One = 10,
    Two = 11,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonInputState {
    Down = 0,
    Up = 1,
}

// TODO: This type is opaque in the C header. Using a byte array as placeholder.
pub type SensorDataRaw = [u8; 64];

// Function pointer types definitions
pub type EventCallback = extern "C" fn(*const c_char);
pub type SensorEventCallback = extern "C" fn(*mut SensorDataRaw);

pub type StartSuduxuFn = unsafe extern "C" fn();
pub type StopSuduxuFn = unsafe extern "C" fn();
pub type IsRunningFn = unsafe extern "C" fn() -> bool;
pub type DisconnectClientFn = unsafe extern "C" fn(c_ushort);
pub type DisconnectAllFn = unsafe extern "C" fn();
pub type GetButtonInStateFn =
    unsafe extern "C" fn(c_ushort, ButtonInputType, ButtonInputState) -> libc::c_int;
pub type GetSensorDataFn = unsafe extern "C" fn(c_ushort) -> SensorDataRaw;
pub type FindAllClientsFn = unsafe extern "C" fn() -> *mut c_char;
pub type FindClientByIdFn = unsafe extern "C" fn(c_ushort) -> *mut c_char;
pub type BroadcastTcpFn = unsafe extern "C" fn(*const c_char);
pub type SendToClientFn = unsafe extern "C" fn(c_ushort, *const c_char);
pub type TickFn = unsafe extern "C" fn(c_float);
pub type FreeFn = unsafe extern "C" fn(*mut c_void);
pub type RegisterEventCallbackFn = unsafe extern "C" fn(EventCallback);
pub type RegisterSensorEventCallbackFn = unsafe extern "C" fn(SensorEventCallback);
