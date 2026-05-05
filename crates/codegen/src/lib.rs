use proc_macro::TokenStream;
use syn::parse_macro_input;

mod expand;
mod parse;

/// `bayesian_filter! { spec = "<path>", name = <Ident> }`
///
/// Reads the spec at macro-expansion time, dispatches on the `ModelSpec`
/// variant, and emits a specialized `update` function for the model class:
///   - Kalman:        struct with `[f32; 4]` state, baked-in K_INF, straight-line update
///   - GammaPoisson:  not yet implemented (compile_error!)
///   - EkfBearing:    not yet implemented (compile_error!)
///
/// Output is dependency-free f32 arithmetic — `cargo expand` will show
/// the inlined matrices and unrolled update body. The embedded crate can
/// pull this in without dragging nalgebra along.
#[proc_macro]
pub fn bayesian_filter(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as parse::MacroInput);
    expand::expand(&parsed).into()
}
