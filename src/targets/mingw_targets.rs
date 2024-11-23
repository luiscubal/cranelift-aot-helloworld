use std::sync::Arc;
use cranelift_codegen::{ir::{AbiParam, Type}, isa::{CallConv, TargetIsa}, settings::Flags};
use cranelift_module::{Module, Linkage};
use target_lexicon::triple;
use anyhow::{Context, Result};
use super::{CStdFunctions, CTypes, Target};

pub struct MingwX64Target {
  ptr_type: Type,
  isa: Arc<dyn TargetIsa>,
  c_types: CTypes,
}

fn requiring_type(candidate_type: Option<Type>, label: &'static str) -> Result<Type> {
  candidate_type.with_context(|| format!("Missing support for C type '{}'", label))
}

impl MingwX64Target {
  pub fn new() -> Result<MingwX64Target> {
    let shared_builder = cranelift_codegen::settings::builder();
    let shared_flags = Flags::new(shared_builder);
    let triple = triple!("x86_64");
    let ptr_type = Type::triple_pointer_type(&triple);
    let builder = cranelift_codegen::isa::lookup(triple).with_context(|| "Could not obtain triple to construct target")?;
    let isa = builder.finish(shared_flags).with_context(|| "Could not create ISA for target")?;

    Ok(MingwX64Target {
      ptr_type,
      isa,
      c_types: CTypes {
        int_type: requiring_type(Type::int(32), "int")?
      },
    })
  }
}

impl Target for MingwX64Target {
  fn get_isa(&self) -> Arc<dyn TargetIsa> {
    self.isa.clone()
  }

  fn get_ptr_type(&self) -> Type {
    self.ptr_type
  }

  fn get_c_interop_types(&self) -> &CTypes {
      &self.c_types
  }

  fn get_c_std_functions(&self, object_module: &mut cranelift_object::ObjectModule) -> Result<CStdFunctions> {
    let mut puts_signature = object_module.make_signature();
    puts_signature.call_conv = CallConv::WindowsFastcall;
    puts_signature.params.push(AbiParam::new(self.get_ptr_type()));
    puts_signature.returns.push(AbiParam::new(self.c_types.int_type));
    let puts_id = object_module.declare_function("puts", Linkage::Import, &puts_signature).with_context(|| "Failed to declare C std function 'puts'")?;

    Ok(CStdFunctions {
      puts: puts_id,
    })
  }
}
