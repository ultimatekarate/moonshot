use std::path::PathBuf;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitStr, Token};

/// `bayesian_filter!{ spec = "<path>", name = <Ident> }`
pub struct MacroInput {
    pub spec_path: PathBuf,
    pub name: Ident,
}

mod kw {
    syn::custom_keyword!(spec);
    syn::custom_keyword!(name);
}

impl Parse for MacroInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // spec = "<path>"
        input.parse::<kw::spec>()?;
        input.parse::<Token![=]>()?;
        let spec_lit: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;

        // name = <Ident>
        input.parse::<kw::name>()?;
        input.parse::<Token![=]>()?;
        let name: Ident = input.parse()?;

        Ok(MacroInput {
            spec_path: PathBuf::from(spec_lit.value()),
            name,
        })
    }
}
