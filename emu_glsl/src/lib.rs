// plan for emu_glsl
// impl Compile for Rust - traverses Vec<Struct | Fn> to generate a glsl compute shader module with 1 entry point
// #[device_fn_mut] - modifies function to return (DeviceFnMut, DeviceFnMutArgs) or DeviceFnMut using Compile on join of used struct's, fn's, fn itself
// #[device_fn] - generates macro_rules that evaluates to a string of the code for fn
// #[derive(Glsl)] - generates macro_rules that evaluates to a string of the code for struct