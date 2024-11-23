mod targets;

use anyhow::{Context, Result};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_codegen::{self, ir::{AbiParam, InstBuilder}};
use cranelift_object::{self, ObjectModule};
use cranelift_module::{self, DataDescription, DataId, Linkage, Module};
use std::fs;

use targets::{CStdFunctions, CTypes, Target};

struct CodegenContext< 'context_life> {
    target: Box<&'context_life dyn Target>,
    c_types: &'context_life CTypes,
    c_functions: CStdFunctions,
    object_module: ObjectModule,
}

fn define_data_literal(ctx: &mut CodegenContext, data_bytes: Box<[u8]>) -> Result<DataId> {
    let literal_id = ctx.object_module.declare_anonymous_data(false, false)?;
    let mut data_description = DataDescription::new();
    data_description.define(data_bytes);
    ctx.object_module.define_data(literal_id, &data_description)?;

    Ok(literal_id)
}

fn generate_code(ctx: &mut CodegenContext) -> Result<()> {
    let mut main_signature = ctx.object_module.make_signature();
    main_signature.params.push(AbiParam::new(ctx.c_types.int_type)); // argc
    main_signature.params.push(AbiParam::new(ctx.target.get_ptr_type())); // argv
    main_signature.returns.push(AbiParam::new(ctx.c_types.int_type));
    let main_func_id = ctx.object_module.declare_function("main", Linkage::Export, &main_signature).unwrap();

    let mut module_context = ctx.object_module.make_context();

    module_context.func.signature = main_signature;
    let mut main_ctx = FunctionBuilderContext::new();
    let mut fn_builder = FunctionBuilder::new(&mut module_context.func, &mut main_ctx);

    let entry_block = fn_builder.create_block();
    fn_builder.append_block_param(entry_block, ctx.c_types.int_type);
    fn_builder.append_block_param(entry_block, ctx.target.get_ptr_type());

    fn_builder.switch_to_block(entry_block);

    let hello_world_id = define_data_literal(ctx, "Hello, World!\0".as_bytes().into())?;
    let hello_world_ref = ctx.object_module.declare_data_in_func(hello_world_id, &mut fn_builder.func);
    let hello_world_ptr = fn_builder.ins().symbol_value(ctx.target.get_ptr_type(), hello_world_ref);
    let puts_ref = ctx.object_module.declare_func_in_func(ctx.c_functions.puts, &mut fn_builder.func);
    fn_builder.ins().call(puts_ref, &[hello_world_ptr]);

    let return_value = 0;
    let result = fn_builder.ins().iconst(ctx.c_types.int_type, return_value);
    fn_builder.ins().return_(& [result]);

    fn_builder.seal_all_blocks();
    fn_builder.finalize();

    ctx.object_module.define_function(main_func_id, &mut module_context)?;

    Ok(())
}

fn main() -> Result<()> {
    let target = targets::mingw_targets::MingwX64Target::new().with_context(|| "Could not initialize target")?;
    let c_types = target.get_c_interop_types();

    let obj_builder = cranelift_object::ObjectBuilder::new(target.get_isa(), "my_module", cranelift_module::default_libcall_names())?;
    let mut object_module = cranelift_object::ObjectModule::new(obj_builder);

    let c_functions = target.get_c_std_functions(&mut object_module)?;

    let mut context = CodegenContext {
        target: Box::new(&target),
        c_types,
        c_functions,
        object_module,
    };

    generate_code(&mut context)?;

    let obj_product = context.object_module.finish();

    let output = obj_product.emit().with_context(|| "Failed to generate binary code")?;
    fs::write("my_module.o", output).with_context(|| "Failed to save module output")?;

    Ok(())
}
