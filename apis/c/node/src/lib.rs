#![deny(unsafe_op_in_unsafe_fn)]
use arrow_array::UInt8Array;
use arrow_array::{Array, Float32Array, Int32Array, UInt64Array};
use dora_node_api::{arrow::array::AsArray, DoraNode, Event, EventStream};
use eyre::Context;
use std::sync::Arc;
use std::{ffi::c_void, ptr, slice};
pub const HEADER_NODE_API: &str = include_str!("../node_api.h");

struct DoraContext {
    node: &'static mut DoraNode,
    events: EventStream,
}

/// Initializes a dora context from the environment variables that were set by
/// the dora-coordinator.
///
/// Returns a pointer to the dora context on success. This pointer can be
/// used to call dora API functions that expect a `context` argument. Any
/// other use is prohibited. To free the dora context when it is no longer
/// needed, use the [`free_dora_context`] function.
///
/// On error, a null pointer is returned.
#[no_mangle]
pub extern "C" fn init_dora_context_from_env() -> *mut c_void {
    let context = || {
        let (node, events) = DoraNode::init_from_env()?;
        let node = Box::leak(Box::new(node));
        Result::<_, eyre::Report>::Ok(DoraContext { node, events })
    };
    let context = match context().context("failed to initialize node") {
        Ok(n) => n,
        Err(err) => {
            let err: eyre::Error = err;
            tracing::error!("{err:?}");
            return ptr::null_mut();
        }
    };

    Box::into_raw(Box::new(context)).cast()
}

/// Frees the given dora context.
///
/// ## Safety
///
/// Only pointers created through [`init_dora_context_from_env`] are allowed
/// as arguments. Each context pointer must be freed exactly once. After
/// freeing, the pointer must not be used anymore.
#[no_mangle]
pub unsafe extern "C" fn free_dora_context(context: *mut c_void) {
    let context: Box<DoraContext> = unsafe { Box::from_raw(context.cast()) };
    // drop all fields except for `node`
    let DoraContext { node, .. } = *context;
    // convert the `'static` reference back to a Box, then drop it
    let _ = unsafe { Box::from_raw(node as *const DoraNode as *mut DoraNode) };
}

/// Waits for the next incoming event for the node.
///
/// Returns a pointer to the event on success. This pointer must not be used
/// directly. Instead, use the `read_dora_event_*` functions to read out the
/// type and payload of the event. When the event is not needed anymore, use
/// [`free_dora_event`] to free it again.
///
/// Returns a null pointer when all event streams were closed. This means that
/// no more event will be available. Nodes typically react by stopping.
///
/// ## Safety
///
/// The `context` argument must be a dora context created through
/// [`init_dora_context_from_env`]. The context must be still valid, i.e., not
/// freed yet.
#[no_mangle]
pub unsafe extern "C" fn dora_next_event(context: *mut c_void) -> *mut c_void {
    let context: &mut DoraContext = unsafe { &mut *context.cast() };
    match context.events.recv() {
        Some(event) => Box::into_raw(Box::new(event)).cast(),
        None => ptr::null_mut(),
    }
}

/// Reads out the type of the given event.
///
/// ## Safety
///
/// The `event` argument must be a dora event received through
/// [`dora_next_event`]. The event must be still valid, i.e., not
/// freed yet.
#[no_mangle]
pub unsafe extern "C" fn read_dora_event_type(event: *const ()) -> EventType {
    let event: &Event = unsafe { &*event.cast() };
    match event {
        Event::Stop => EventType::Stop,
        Event::Input { .. } => EventType::Input,
        Event::InputClosed { .. } => EventType::InputClosed,
        Event::Error(_) => EventType::Error,
        _ => EventType::Unknown,
    }
}

#[repr(C)]
pub enum EventType {
    Stop,
    Input,
    InputClosed,
    Error,
    Unknown,
}

/// Reads out the ID of the given input event.
///
/// Writes the `out_ptr` and `out_len` with the start pointer and length of the
/// ID string of the input. The ID is guaranteed to be valid UTF-8.
///
/// Writes a null pointer and length `0` if the given event is not an input event.
///
/// ## Safety
///
/// The `event` argument must be a dora event received through
/// [`dora_next_event`]. The event must be still valid, i.e., not
/// freed yet. The returned `out_ptr` must not be used after
/// freeing the `event`, since it points directly into the event's
/// memory.
#[no_mangle]
pub unsafe extern "C" fn read_dora_input_id(
    event: *const (),
    out_ptr: *mut *const u8,
    out_len: *mut usize,
) {
    let event: &Event = unsafe { &*event.cast() };
    match event {
        Event::Input { id, .. } => {
            let id = id.as_str().as_bytes();
            let ptr = id.as_ptr();
            let len = id.len();
            unsafe {
                *out_ptr = ptr;
                *out_len = len;
            }
        }
        _ => unsafe {
            *out_ptr = ptr::null();
            *out_len = 0;
        },
    }
}

#[no_mangle]
pub unsafe extern "C" fn read_dora_input_data_u8(
    event: *const (),
    out_ptr: *mut *const u8,
    out_len: *mut usize,
) {
    let event: &Event = unsafe { &*event.cast() };
    match event {
        Event::Input { data, metadata, .. } => match metadata.type_info.data_type {
            dora_node_api::arrow::datatypes::DataType::UInt8 => {
                let array: &UInt8Array = data.as_primitive();
                let ptr = array.values().as_ptr();
                unsafe {
                    *out_ptr = ptr;
                    *out_len = metadata.type_info.len;
                }
            }
            dora_node_api::arrow::datatypes::DataType::Null => unsafe {
                *out_ptr = ptr::null();
                *out_len = 0;
            },
            _ => {
                panic!("You used {}, must use U8!", metadata.type_info.data_type);
            }
        },
        _ => unsafe {
            *out_ptr = ptr::null();
            *out_len = 0;
        },
    }
}

#[no_mangle]
pub unsafe extern "C" fn read_dora_input_data_i32(
    event: *const (),
    out_ptr: *mut *const i32,
    out_len: *mut usize,
) {
    let event: &Event = unsafe { &*event.cast() };
    match event {
        Event::Input { data, metadata, .. } => match metadata.type_info.data_type {
            dora_node_api::arrow::datatypes::DataType::Int32 => {
                let array: &Int32Array = data.as_primitive();
                let ptr = array.values().as_ptr();
                unsafe {
                    *out_ptr = ptr;
                    *out_len = metadata.type_info.len;
                }
            }
            dora_node_api::arrow::datatypes::DataType::Null => unsafe {
                *out_ptr = ptr::null();
                *out_len = 0;
            },
            _ => {
                panic!("You used {}, must use Int32!", metadata.type_info.data_type);
            }
        },
        _ => unsafe {
            *out_ptr = ptr::null();
            *out_len = 0;
        },
    }
}

#[no_mangle]
pub unsafe extern "C" fn read_dora_input_data_f32(
    event: *const (),
    out_ptr: *mut *const f32,
    out_len: *mut usize,
) {
    let event: &Event = unsafe { &*event.cast() };

    match event {
        Event::Input { data, metadata, .. } => match metadata.type_info.data_type {
            dora_node_api::arrow::datatypes::DataType::Float32 => {
                let array: &Float32Array = data.as_primitive();
                let ptr = array.values().as_ptr();
                unsafe {
                    *out_ptr = ptr;
                    *out_len = metadata.type_info.len;
                }
            }
            dora_node_api::arrow::datatypes::DataType::Null => unsafe {
                *out_ptr = ptr::null();
                *out_len = 0;
            },
            _ => {
                panic!(
                    "You used {}, must use Float32!",
                    metadata.type_info.data_type
                );
            }
        },
        _ => unsafe {
            *out_ptr = ptr::null();
            *out_len = 0;
        },
    }
}

#[no_mangle]
pub unsafe extern "C" fn read_dora_input_data_u64(
    event: *const (),
    out_ptr: *mut *const u64,
    out_len: *mut usize,
) {
    let event: &Event = unsafe { &*event.cast() };

    match event {
        Event::Input { data, metadata, .. } => match metadata.type_info.data_type {
            dora_node_api::arrow::datatypes::DataType::UInt64 => {
                let array: &UInt64Array = data.as_primitive();
                let ptr = array.values().as_ptr();
                unsafe {
                    *out_ptr = ptr;
                    *out_len = metadata.type_info.len;
                }
            }
            dora_node_api::arrow::datatypes::DataType::Null => unsafe {
                *out_ptr = ptr::null();
                *out_len = 0;
            },
            _ => {
                panic!(
                    "You used {}, must use UInt64!",
                    metadata.type_info.data_type
                );
            }
        },
        _ => unsafe {
            *out_ptr = ptr::null();
            *out_len = 0;
        },
    }
}

/// Frees the given dora event.
///
/// ## Safety
///
/// Only pointers created through [`dora_next_event`] are allowed
/// as arguments. Each context pointer must be freed exactly once. After
/// freeing, the pointer and all derived pointers must not be used anymore.
/// This also applies to the `read_dora_event_*` functions, which return
/// pointers into the original event structure.
#[no_mangle]
pub unsafe extern "C" fn free_dora_event(event: *mut c_void) {
    let _: Box<Event> = unsafe { Box::from_raw(event.cast()) };
}

#[no_mangle]
pub unsafe extern "C" fn dora_send_output_u8(
    context: *mut c_void,
    id_ptr: *const u8,
    id_len: usize,
    data_ptr: *const u8,
    data_len: usize,
) -> isize {
    match unsafe { try_send_output(context, id_ptr, id_len, data_ptr, data_len) } {
        Ok(()) => 0,
        Err(err) => {
            tracing::error!("{err:?}");
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn dora_send_output_i32(
    context: *mut c_void,
    id_ptr: *const u8,
    id_len: usize,
    data_ptr: *const i32,
    data_len: usize,
) -> isize {
    match unsafe { try_send_output(context, id_ptr, id_len, data_ptr, data_len) } {
        Ok(()) => 0,
        Err(err) => {
            tracing::error!("{err:?}");
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn dora_send_output_f32(
    context: *mut c_void,
    id_ptr: *const u8,
    id_len: usize,
    data_ptr: *const f32,
    data_len: usize,
) -> isize {
    match unsafe { try_send_output(context, id_ptr, id_len, data_ptr, data_len) } {
        Ok(()) => 0,
        Err(err) => {
            tracing::error!("{err:?}");
            -1
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn dora_send_output_u64(
    context: *mut c_void,
    id_ptr: *const u8,
    id_len: usize,
    data_ptr: *const u64,
    data_len: usize,
) -> isize {
    match unsafe { try_send_output(context, id_ptr, id_len, data_ptr, data_len) } {
        Ok(()) => 0,
        Err(err) => {
            tracing::error!("{err:?}");
            -1
        }
    }
}

pub trait ToArrow {
    fn to_arrow(self) -> Arc<dyn Array>;
}

impl ToArrow for &[f32] {
    fn to_arrow(self) -> Arc<dyn Array> {
        let array = Float32Array::from(self.to_vec());
        Arc::new(array)
    }
}

impl ToArrow for &[i32] {
    fn to_arrow(self) -> Arc<dyn Array> {
        let array = Int32Array::from(self.to_vec());
        Arc::new(array)
    }
}

impl ToArrow for &[u64] {
    fn to_arrow(self) -> Arc<dyn Array> {
        let array = UInt64Array::from(self.to_vec());
        Arc::new(array)
    }
}

impl ToArrow for &[u8] {
    fn to_arrow(self) -> Arc<dyn Array> {
        let array = UInt8Array::from(self.to_vec());
        Arc::new(array)
    }
}

unsafe fn try_send_output<T>(
    context: *mut c_void,
    id_ptr: *const u8,
    id_len: usize,
    data_ptr: *const T,
    data_len: usize,
) -> eyre::Result<()>
where
    for<'a> &'a [T]: ToArrow,
{
    let context: &mut DoraContext = unsafe { &mut *context.cast() };
    let id = std::str::from_utf8(unsafe { slice::from_raw_parts(id_ptr, id_len) })?;
    let output_id = id.to_owned().into();

    let data = unsafe { slice::from_raw_parts(data_ptr, data_len) };
    let data_array = data.to_arrow();
    context
        .node
        .send_output(output_id, Default::default(), data_array)
}
