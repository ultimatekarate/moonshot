use proc_macro::TokenStream;
use quote::quote;

mod expand;
mod parse;

/// `bayesian_filter! { spec = "../../model.ron", name = Filter4x2 }`
///
/// Single entry point that reads the spec, dispatches on `ModelSpec` variant,
/// and emits a specialized `update` function appropriate to the model class:
///   - Kalman:        struct with `[f32; 4]` state, baked-in K_INF, straight-line update
///   - GammaPoisson:  struct with `(α, β)`, integer-update method
///   - EkfBearing:    struct with `[f32; 4]` state, runtime-Jacobian update
///
/// Stub: emits an empty Filter4x2 so codegen-demo compiles. See docs/plan.md Lab 4.
#[proc_macro]
pub fn bayesian_filter(_input: TokenStream) -> TokenStream {
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
