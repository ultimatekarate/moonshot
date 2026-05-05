use proc_macro::TokenStream;
use quote::quote;

mod expand;
mod parse;

/// `kalman_filter! { spec = "../../model.ron", name = Filter4x2 }`
///
/// Stub: emits an empty struct so codegen-demo compiles.
/// Real implementation reads the spec via spec-loader, bakes in the
/// Riccati gain, emits straight-line arithmetic. See docs/plan.md Lab 4.
#[proc_macro]
pub fn kalman_filter(_input: TokenStream) -> TokenStream {
    quote! {
        pub struct Filter4x2 {
            pub x: [f32; 4],
        }

        impl Filter4x2 {
            pub const fn new() -> Self {
                Self { x: [0.0; 4] }
            }

            pub fn update(&mut self, _z_x: f32, _z_y: f32) {
                // stub
            }
        }
    }
    .into()
}
