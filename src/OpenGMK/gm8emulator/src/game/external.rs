pub mod dll;
mod dummy;
pub mod roz;
pub mod win32;
mod wow64;

use crate::{gml, gml::Function, types::ID};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
use dummy as native;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
use win32 as native;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use dummy as ipc;
#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
use wow64 as ipc;

pub use native::NativeExternal;

pub enum Call {
    Dummy(gml::Value),
    Emulated(Function),
    Native(NativeExternal),
    Ipc(ipc::IpcExternal),
    Roz(roz::RozFn),
}

pub struct External {
    pub call: Call,
    pub signature: dll::ExternalSignature,
}

pub struct ExternalManager {
    externals: Vec<Option<External>>,
    dummy_audio: bool,
    roz: roz::Roz,
    roz_used: bool,

    native_manager: native::NativeManager,
    ipc_manager: ipc::IpcManager,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ExternalState {
    signatures: Vec<Option<dll::ExternalSignature>>,
}

impl ExternalManager {
    pub fn new(dummy_audio: bool) -> Self {
        Self {
            externals: Vec::new(),
            dummy_audio,
            roz: roz::Roz::new(),
            roz_used: false,
            native_manager: native::NativeManager::new(),
            ipc_manager: ipc::IpcManager::new(),
        }
    }

    /// True once the loaded game imports ROZ.dll, i.e. it is the developer's Online build, whose
    /// own Multijugador menu (with the internet option) must not be replaced by the overlay.
    pub fn roz_used(&self) -> bool {
        self.roz_used
    }

    fn make_call(&mut self, signature: &dll::ExternalSignature) -> Result<Call, String> {
        if let Some(dummy) = self.should_dummy(&signature) {
            return Ok(Call::Dummy(dummy))
        }
        // ROZ.dll (Reloaded Online) is emulated natively on every platform.
        let file = Path::new(&signature.dll).file_name().and_then(|o| o.to_str()).unwrap_or(&signature.dll);
        if file.eq_ignore_ascii_case("roz.dll") {
            self.roz_used = true;
            // An export the shim doesn't know returns 0 instead of aborting the game.
            return Ok(match roz::RozFn::from_symbol(&signature.symbol) {
                Some(f) => Call::Roz(f),
                None => Call::Dummy(gml::Value::Real(0.into())),
            })
        }
        if cfg!(all(target_os = "windows", target_arch = "x86")) {
            Ok(Call::Native(self.native_manager.define(&signature)?))
        } else {
            match self.ipc_manager.define(&signature) {
                Ok(ipc) => Ok(Call::Ipc(ipc)),
                // Android cannot load Windows DLLs: a failed define used to end the game at
                // launch. Answer 0 to every call instead, as the audio dummies already do.
                Err(e) if cfg!(target_os = "android") => {
                    eprintln!("[external] {}::{} unavailable here ({e}); returning 0", signature.dll, signature.symbol);
                    Ok(Call::Dummy(gml::Value::Real(0.into())))
                },
                Err(e) => Err(e),
            }
        }
    }

    pub fn define(&mut self, signature: dll::ExternalSignature) -> Result<ID, String> {
        let external = External { call: self.make_call(&signature)?, signature };
        if let Some((id, cell)) = self.externals.iter_mut().enumerate().find(|(_, o)| o.is_none()) {
            *cell = Some(external);
            Ok(id as ID)
        } else {
            let id = self.externals.len();
            self.externals.push(Some(external));
            Ok(id as ID)
        }
    }

    pub fn get_external(&self, id: i32) -> Option<&External> {
        id.try_into().ok().and_then(|id: usize| self.externals.get(id).map(Option::as_ref).flatten())
    }

    pub fn call_native(&mut self, id: usize, args: &[dll::Value]) -> gml::Value {
        match &self.externals[id].as_ref().unwrap().call {
            Call::Native(ext) => self.native_manager.call(ext, args).into(),
            _ => unreachable!(),
        }
    }

    pub fn call_roz(&mut self, id: usize, args: &[dll::Value]) -> gml::Value {
        match &self.externals[id].as_ref().unwrap().call {
            &Call::Roz(f) => self.roz.call(f, args).into(),
            _ => unreachable!(),
        }
    }

    pub fn call_ipc(&mut self, id: usize, args: &[dll::Value]) -> gml::Value {
        match &self.externals[id].as_ref().unwrap().call {
            Call::Ipc(ext) => self.ipc_manager.call(ext, args).into(),
            _ => unreachable!(),
        }
    }

    pub fn free(&mut self, dll: &str) {
        for option in &mut self.externals {
            if let Some(external) = option.as_ref() {
                if external.signature.dll.eq_ignore_ascii_case(dll) {
                    // ipc externals need the manager so free those with that
                    match option.take().unwrap().call {
                        Call::Ipc(e) => self.ipc_manager.free(e),
                        _ => (),
                    }
                }
            }
        }
    }

    pub fn save_state(&self) -> ExternalState {
        let signatures = self.externals.iter().map(|o| o.as_ref().map(|e| e.signature.clone())).collect();
        ExternalState { signatures }
    }

    pub fn load_state(&mut self, mut state: ExternalState) {
        self.externals.clear();
        for opt in state.signatures.drain(..) {
            let external = opt.map(|s| External { call: self.make_call(&s).unwrap(), signature: s });
            self.externals.push(external);
        }
    }

    fn should_dummy(&self, signature: &dll::ExternalSignature) -> Option<gml::Value> {
        let dll = &signature.dll;
        let sym = &signature.symbol;
        let dll = Path::new(dll).file_name().and_then(|oss| oss.to_str()).unwrap_or(dll);

        let mut dummy = None;
        if self.dummy_audio {
            if dll.eq_ignore_ascii_case("gmfmodsimple.dll") {
                if sym == "FMODSoundAdd" {
                    dummy = Some(gml::Value::Real(1.into()));
                } else {
                    dummy = Some(gml::Value::Real(0.into()));
                }
            } else if dll.eq_ignore_ascii_case("ssound.dll") || dll.eq_ignore_ascii_case("supersound.dll") {
                if sym == "SS_Init" {
                    dummy = Some(gml::Value::Str("Yes".into()));
                } else {
                    dummy = Some(gml::Value::Real(0.into()));
                }
            } else if dll.eq_ignore_ascii_case("sgaudio.dll") || dll.eq_ignore_ascii_case("sxms-3.dll") {
                dummy = Some(gml::Value::Real(0.into()));
            } else if dll.eq_ignore_ascii_case("caster.dll") {
                if sym == "caster_error_message" || sym == "caster_version" {
                    dummy = Some(gml::Value::Str("".into()));
                } else {
                    dummy = Some(gml::Value::Real(0.into()));
                }
            }
        }

        if dll.eq_ignore_ascii_case("gmeffect_0.1.dll") {
            // TODO: don't
            // ^ floogle's original comment, whatever it may mean
            // ^ viri's original comment, made without consulting floogle
            dummy = Some(gml::Value::Real(0.into()));
        }

        dummy
    }
}
