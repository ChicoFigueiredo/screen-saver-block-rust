//! macOS: a tela fica ativa por uma asserção de energia do IOKit, e o logout,
//! a reinicialização e o desligamento são cancelados pelo
//! `applicationShouldTerminate:` do delegate do `NSApplication`.

use std::sync::atomic::{AtomicBool, Ordering};

use core_foundation::{
    base::TCFType,
    string::{CFString, CFStringRef},
};
use objc2::{
    MainThreadMarker, class, ffi, msg_send,
    runtime::{AnyObject, Imp, Sel},
    sel,
};

const ASSERTION_NAME: &str = "Block Screen Saver: bloqueio solicitado pelo usuário";
/// Impede o protetor de tela, o escurecimento e o repouso da tela por
/// inatividade, e com eles o bloqueio automático da sessão.
const DISPLAY_ASSERTION: &str = "PreventUserIdleDisplaySleep";

type IOPMAssertionID = u32;
type IOReturn = i32;
const IOPM_ASSERTION_LEVEL_ON: u32 = 255;
const IO_RETURN_SUCCESS: IOReturn = 0;

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOPMAssertionCreateWithName(
        assertion_type: CFStringRef,
        level: u32,
        name: CFStringRef,
        id: *mut IOPMAssertionID,
    ) -> IOReturn;
    fn IOPMAssertionRelease(id: IOPMAssertionID) -> IOReturn;
}

/// Lido pelo `applicationShouldTerminate:`, que o AppKit chama sem contexto.
static SESSION_BLOCKED: AtomicBool = AtomicBool::new(false);

#[derive(Default)]
pub struct PlatformInhibitor {
    display_assertion: Option<IOPMAssertionID>,
    terminate_hook: bool,
}

impl PlatformInhibitor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_screen_saver(&mut self, enabled: bool) -> Result<(), String> {
        if enabled {
            if self.display_assertion.is_some() {
                return Ok(());
            }
            let assertion_type = CFString::new(DISPLAY_ASSERTION);
            let name = CFString::new(ASSERTION_NAME);
            let mut id: IOPMAssertionID = 0;
            // SAFETY: as duas CFStrings vivem até o fim da chamada, e `id` aponta
            // para memória válida.
            let result = unsafe {
                IOPMAssertionCreateWithName(
                    assertion_type.as_concrete_TypeRef(),
                    IOPM_ASSERTION_LEVEL_ON,
                    name.as_concrete_TypeRef(),
                    &mut id,
                )
            };
            if result != IO_RETURN_SUCCESS {
                return Err(format!("IOPMAssertionCreateWithName falhou: {result:#x}"));
            }
            self.display_assertion = Some(id);
        } else if let Some(id) = self.display_assertion.take() {
            // SAFETY: `id` veio de `IOPMAssertionCreateWithName` e é liberado uma vez só.
            let result = unsafe { IOPMAssertionRelease(id) };
            if result != IO_RETURN_SUCCESS {
                return Err(format!("IOPMAssertionRelease falhou: {result:#x}"));
            }
        }
        Ok(())
    }

    pub fn set_session(&mut self, enabled: bool) -> Result<(), String> {
        if enabled && !self.terminate_hook {
            install_terminate_hook()?;
            self.terminate_hook = true;
        }
        SESSION_BLOCKED.store(enabled, Ordering::SeqCst);
        Ok(())
    }
}

impl Drop for PlatformInhibitor {
    fn drop(&mut self) {
        let _ = self.set_session(false);
        let _ = self.set_screen_saver(false);
    }
}

// Valores de `NSApplicationTerminateReply`.
const TERMINATE_CANCEL: usize = 0;
const TERMINATE_NOW: usize = 1;

const fn four_char_code(code: &[u8; 4]) -> u32 {
    u32::from_be_bytes(*code)
}

// Códigos dos Apple Events (AE/AppleEvents.h e AE/AERegistry.h).
const CORE_EVENT_CLASS: u32 = four_char_code(b"aevt");
const AE_QUIT_APPLICATION: u32 = four_char_code(b"quit");
const AE_QUIT_REASON: u32 = four_char_code(b"why?");
/// Motivos com que o `loginwindow` pede para o app sair: logout, reinicialização
/// e desligamento, com ou sem a janela de confirmação.
const SYSTEM_QUIT_REASONS: [u32; 6] = [
    four_char_code(b"logo"), // kAELogOut
    four_char_code(b"rlgo"), // kAEReallyLogOut
    four_char_code(b"rrst"), // kAEShowRestartDialog
    four_char_code(b"rsdn"), // kAEShowShutdownDialog
    four_char_code(b"rest"), // kAERestart
    four_char_code(b"shut"), // kAEShutDown
];

/// O winit registra o próprio delegate (`WinitApplicationDelegate`), que não
/// implementa `applicationShouldTerminate:`. O método é acrescentado à classe
/// dele em tempo de execução. Só é possível com a janela aberta, quando o delegate
/// já existe.
fn install_terminate_hook() -> Result<(), String> {
    if MainThreadMarker::new().is_none() {
        return Err("o bloqueio de sessão precisa ser ligado pela janela".to_owned());
    }
    // SAFETY: estamos na thread principal, exigida pelo AppKit; os objetos vêm
    // do próprio `NSApplication` e o método segue a assinatura do protocolo.
    unsafe {
        let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let delegate: *mut AnyObject = msg_send![app, delegate];
        if delegate.is_null() {
            return Err("a janela ainda não terminou de abrir".to_owned());
        }
        let selector = sel!(applicationShouldTerminate:);
        let responds: bool = msg_send![delegate, respondsToSelector: selector];
        if responds {
            return Err("o delegate do aplicativo já trata o encerramento".to_owned());
        }

        let class = (*delegate).class() as *const _ as *mut _;
        let imp: Imp = std::mem::transmute(
            should_terminate
                as unsafe extern "C-unwind" fn(*mut AnyObject, Sel, *mut AnyObject) -> usize,
        );
        // Retorna NSUInteger e recebe self, _cmd e o NSApplication.
        if !ffi::class_addMethod(class, selector, imp, c"Q@:@".as_ptr()).as_bool() {
            return Err("não foi possível registrar o tratamento do logout".to_owned());
        }
        // O AppKit guarda, no `setDelegate:`, quais métodos opcionais o delegate
        // implementa; reatribuir o mesmo objeto faz ele enxergar o novo método.
        let _: () = msg_send![app, setDelegate: delegate];
    }
    Ok(())
}

unsafe extern "C-unwind" fn should_terminate(
    _this: *mut AnyObject,
    _cmd: Sel,
    _sender: *mut AnyObject,
) -> usize {
    // Sair pelo menu (⌘Q) ou pelo Dock continua permitido; só o encerramento
    // pedido pelo sistema é recusado.
    if SESSION_BLOCKED.load(Ordering::SeqCst) && quit_requested_by_system() {
        TERMINATE_CANCEL
    } else {
        TERMINATE_NOW
    }
}

/// O logout chega como o Apple Event `quit` com o atributo `kAEQuitReason`,
/// que ainda é o evento corrente quando o AppKit consulta o delegate.
fn quit_requested_by_system() -> bool {
    // SAFETY: chamado pelo AppKit na thread principal; ponteiros nulos são tratados.
    unsafe {
        let manager: *mut AnyObject =
            msg_send![class!(NSAppleEventManager), sharedAppleEventManager];
        let event: *mut AnyObject = msg_send![manager, currentAppleEvent];
        if event.is_null() {
            return false;
        }
        let event_class: u32 = msg_send![event, eventClass];
        let event_id: u32 = msg_send![event, eventID];
        if event_class != CORE_EVENT_CLASS || event_id != AE_QUIT_APPLICATION {
            return false;
        }
        let reason: *mut AnyObject =
            msg_send![event, attributeDescriptorForKeyword: AE_QUIT_REASON];
        if reason.is_null() {
            return false;
        }
        let reason: u32 = msg_send![reason, typeCodeValue];
        SYSTEM_QUIT_REASONS.contains(&reason)
    }
}
