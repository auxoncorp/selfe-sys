#![no_std]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

#[macro_use]
extern crate static_assertions;

type seL4_CPtr = usize;
type seL4_Word = usize;
type seL4_Int8 = i8;
type seL4_Int16 = i16;
type seL4_Int32 = i32;
type seL4_Int64 = i64;
type seL4_Uint8 = u8;
type seL4_Uint16 = u16;
type seL4_Uint32 = u32;
type seL4_Uint64 = u64;

#[cfg(any(target_arch = "arm", target_arch = "x86"))]
mod ctypes {
    pub type c_char = i8;
    pub type c_uint = u32;
    pub type c_int = i32;
    pub type c_ulong = u32;
}

#[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
pub mod ctypes {
    pub type c_char = i8;
    pub type c_uint = u32;
    pub type c_int = i32;
    pub type c_ulong = u64;
}

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

mod compile_time_assertions {
    use super::*;

    assert_eq_size!(capdata_is_one_word; seL4_Word, seL4_CNode_CapData);
    assert_eq_size!(caprights_is_one_word; seL4_Word, seL4_CapRights);
    assert_eq_size!(message_info_is_one_word; seL4_Word, seL4_MessageInfo);
    assert_eq_size!(user_context_is_defined; seL4_UserContext, seL4_UserContext);
    assert_eq_size!(fault_is_defined; seL4_Fault, seL4_Fault);
    assert_eq_size!(ipc_buffer_is_defined; seL4_IPCBuffer, seL4_IPCBuffer);

    // Core common functions that are not syscalls
    const SEL4_GETMR: unsafe extern "C" fn(ctypes::c_int) -> seL4_Word = seL4_GetMR;
    const SEL4_SETMR: unsafe extern "C" fn(ctypes::c_int, seL4_Word) = seL4_SetMR;
    const SEL4_GETUSERDATA: unsafe extern "C" fn() -> seL4_Word = seL4_GetUserData;
    const SEL4_SETUSERDATA: unsafe extern "C" fn(seL4_Word) = seL4_SetUserData;
    const SEL4_GETBADGE: unsafe extern "C" fn(ctypes::c_int) -> seL4_Word = seL4_GetBadge;
    const SEL4_GETCAP: unsafe extern "C" fn(ctypes::c_int) -> seL4_CPtr = seL4_GetCap;
    const SEL4_SETCAP: unsafe extern "C" fn(ctypes::c_int, seL4_CPtr) = seL4_SetCap;
    const SEL4_GETIPCBUFFER: unsafe extern "C" fn() -> *mut seL4_IPCBuffer = seL4_GetIPCBuffer;

    // Syscalls
    const SEL4_SEND: unsafe extern "C" fn(seL4_CPtr, seL4_MessageInfo) = seL4_Send;
    const SEL4_NBSEND: unsafe extern "C" fn(seL4_CPtr, seL4_MessageInfo) = seL4_NBSend;
    const SEL4_REPLY: unsafe extern "C" fn(seL4_MessageInfo) = seL4_Reply;
    const SEL4_SIGNAL: unsafe extern "C" fn(seL4_CPtr) = seL4_Signal;
    const SEL4_RECV: unsafe extern "C" fn(seL4_CPtr, *mut seL4_Word) -> seL4_MessageInfo =
        seL4_Recv;
    const SEL4_NBRECV: unsafe extern "C" fn(seL4_CPtr, *mut seL4_Word) -> seL4_MessageInfo =
        seL4_NBRecv;
    const SEL4_CALL: unsafe extern "C" fn(seL4_CPtr, seL4_MessageInfo) -> seL4_MessageInfo =
        seL4_Call;
    const SEL4_REPLYRECV: unsafe extern "C" fn(
        seL4_CPtr,
        seL4_MessageInfo,
        *mut seL4_Word,
    ) -> seL4_MessageInfo = seL4_ReplyRecv;
    const SEL4_YIELD: unsafe extern "C" fn() = seL4_Yield;
    const SEL4_WAIT: unsafe extern "C" fn(seL4_CPtr, *mut seL4_Word) = seL4_Wait;
    const SEL4_POLL: unsafe extern "C" fn(seL4_CPtr, *mut seL4_Word) -> seL4_MessageInfo =
        seL4_Poll;

    // API object CPtrs
    assert_eq_size!(cptr_cnode; seL4_CPtr, seL4_CNode);
    assert_eq_size!(cptr_irq_handler; seL4_CPtr, seL4_IRQHandler);
    assert_eq_size!(cptr_irq_control; seL4_CPtr, seL4_IRQControl);
    assert_eq_size!(cptr_tcb; seL4_CPtr, seL4_TCB);
    assert_eq_size!(cptr_untyped; seL4_CPtr, seL4_Untyped);
    assert_eq_size!(cptr_domain_set; seL4_CPtr, seL4_DomainSet);

    assert_eq_size!(error_is_defined; seL4_Error, seL4_Error);
    assert_eq_size!(bool_is_defined; seL4_Bool, seL4_Bool);
    assert_eq_size!(bootinfo_is_defined; seL4_BootInfo, seL4_BootInfo);

    // Target-independent API functions
    const UNTYPED_RETYPE: unsafe extern "C" fn(
        seL4_Untyped,
        seL4_Word,
        seL4_Word,
        seL4_CNode,
        seL4_Word,
        seL4_Word,
        seL4_Word,
        seL4_Word,
    ) -> seL4_Error = seL4_Untyped_Retype;
    const TCB_READREGISTERS: unsafe extern "C" fn(
        seL4_TCB,
        seL4_Bool,
        seL4_Uint8,
        seL4_Word,
        *mut seL4_UserContext,
    ) -> seL4_Error = seL4_TCB_ReadRegisters;
    const TCB_WRITEREGISTERS: unsafe extern "C" fn(
        seL4_TCB,
        seL4_Bool,
        seL4_Uint8,
        seL4_Word,
        *mut seL4_UserContext,
    ) -> seL4_Error = seL4_TCB_WriteRegisters;
    const TCB_COPYREGISTERS: unsafe extern "C" fn(
        seL4_TCB,
        seL4_TCB,
        seL4_Bool,
        seL4_Bool,
        seL4_Bool,
        seL4_Bool,
        seL4_Uint8,
    ) -> seL4_Error = seL4_TCB_CopyRegisters;
    const TCB_CONFIGURE: unsafe extern "C" fn(
        seL4_TCB,
        seL4_Word,
        seL4_CNode,
        seL4_Word,
        seL4_CNode,
        seL4_Word,
        seL4_Word,
        seL4_CPtr,
    ) -> seL4_Error = seL4_TCB_Configure;
    const TCB_SETPRIORITY: unsafe extern "C" fn(seL4_TCB, seL4_CPtr, seL4_Word) -> seL4_Error =
        seL4_TCB_SetPriority;
    const TCB_SETMCPRIORITY: unsafe extern "C" fn(seL4_TCB, seL4_CPtr, seL4_Word) -> seL4_Error =
        seL4_TCB_SetMCPriority;
    const TCB_SETSCHEDPARAMS: unsafe extern "C" fn(
        seL4_TCB,
        seL4_CPtr,
        seL4_Word,
        seL4_Word,
    ) -> seL4_Error = seL4_TCB_SetSchedParams;
    const TCB_SETIPCBUFFER: unsafe extern "C" fn(seL4_TCB, seL4_Word, seL4_CPtr) -> seL4_Error =
        seL4_TCB_SetIPCBuffer;
    const TCB_SETSPACE: unsafe extern "C" fn(
        seL4_TCB,
        seL4_Word,
        seL4_CNode,
        seL4_Word,
        seL4_CNode,
        seL4_Word,
    ) -> seL4_Error = seL4_TCB_SetSpace;
    const TCB_SUSPEND: unsafe extern "C" fn(seL4_TCB) -> seL4_Error = seL4_TCB_Suspend;
    const TCB_RESUME: unsafe extern "C" fn(seL4_TCB) -> seL4_Error = seL4_TCB_Resume;
    const TCB_BINDNOTIFICATION: unsafe extern "C" fn(seL4_TCB, seL4_CPtr) -> seL4_Error =
        seL4_TCB_BindNotification;
    const TCB_UNBINDNOTIFICATION: unsafe extern "C" fn(seL4_TCB) -> seL4_Error =
        seL4_TCB_UnbindNotification;
    const CNODE_REVOKE: unsafe extern "C" fn(seL4_CNode, seL4_Word, seL4_Uint8) -> seL4_Error =
        seL4_CNode_Revoke;
    const CNODE_DELETE: unsafe extern "C" fn(seL4_CNode, seL4_Word, seL4_Uint8) -> seL4_Error =
        seL4_CNode_Delete;
    const CNODE_CANCELBADGEDSENDS: unsafe extern "C" fn(
        seL4_CNode,
        seL4_Word,
        seL4_Uint8,
    ) -> seL4_Error = seL4_CNode_CancelBadgedSends;
    const CNODE_COPY: unsafe extern "C" fn(
        seL4_CNode,
        seL4_Word,
        seL4_Uint8,
        seL4_CNode,
        seL4_Word,
        seL4_Uint8,
        seL4_CapRights,
    ) -> seL4_Error = seL4_CNode_Copy;
    const CNODE_MINT: unsafe extern "C" fn(
        seL4_CNode,
        seL4_Word,
        seL4_Uint8,
        seL4_CNode,
        seL4_Word,
        seL4_Uint8,
        seL4_CapRights,
        seL4_Word,
    ) -> seL4_Error = seL4_CNode_Mint;
    const CNODE_MOVE: unsafe extern "C" fn(
        seL4_CNode,
        seL4_Word,
        seL4_Uint8,
        seL4_CNode,
        seL4_Word,
        seL4_Uint8,
    ) -> seL4_Error = seL4_CNode_Move;
    const CNODE_MUTATE: unsafe extern "C" fn(
        seL4_CNode,
        seL4_Word,
        seL4_Uint8,
        seL4_CNode,
        seL4_Word,
        seL4_Uint8,
        seL4_Word,
    ) -> seL4_Error = seL4_CNode_Mutate;
    const CNODE_ROTATE: unsafe extern "C" fn(
        seL4_CNode,
        seL4_Word,
        seL4_Uint8,
        seL4_Word,
        seL4_CNode,
        seL4_Word,
        seL4_Uint8,
        seL4_Word,
        seL4_CNode,
        seL4_Word,
        seL4_Uint8,
    ) -> seL4_Error = seL4_CNode_Rotate;
    const CNODE_SAVECALLER: unsafe extern "C" fn(seL4_CNode, seL4_Word, seL4_Uint8) -> seL4_Error =
        seL4_CNode_SaveCaller;
    const IRQCONTROL_GET: unsafe extern "C" fn(
        seL4_IRQControl,
        ctypes::c_int,
        seL4_CNode,
        seL4_Word,
        seL4_Uint8,
    ) -> seL4_Error = seL4_IRQControl_Get;
    const IRQHANDLER_ACK: unsafe extern "C" fn(seL4_IRQHandler) -> seL4_Error = seL4_IRQHandler_Ack;
    const IRQHANDLER_SETNOTIFICATION: unsafe extern "C" fn(
        seL4_IRQHandler,
        seL4_CPtr,
    ) -> seL4_Error = seL4_IRQHandler_SetNotification;
    const IRQHANDLER_CLEAR: unsafe extern "C" fn(seL4_IRQHandler) -> seL4_Error =
        seL4_IRQHandler_Clear;
    const DOMAINSET_SET: unsafe extern "C" fn(seL4_DomainSet, seL4_Uint8, seL4_TCB) -> seL4_Error =
        seL4_DomainSet_Set;

    // TODO - constants of interest, e.g. the retype-ids for arch-agnostic kernel objects
    // TODO - x86 and arm specific structures and functions
}
