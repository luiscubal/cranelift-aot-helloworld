use std::sync::Arc;

use anyhow::Result;
use cranelift_codegen::{ir::Type, isa::TargetIsa};
use cranelift_module::FuncId;
use cranelift_object::ObjectModule;

pub mod mingw_targets;

pub struct CTypes {
  pub int_type: Type,
}

pub struct CStdFunctions {
  pub puts: FuncId,
}

pub trait Target {
  fn get_isa(&self) -> Arc<dyn TargetIsa>;

  fn get_ptr_type(&self) -> Type;

  fn get_c_interop_types(&self) -> &CTypes;

  fn get_c_std_functions(&self, object_module: &mut ObjectModule) -> Result<CStdFunctions>;
}
