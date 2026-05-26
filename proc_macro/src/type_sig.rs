use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{braced, custom_keyword, Expr, Token, Type};
use syn::parse::{Parse, ParseStream};

#[derive(Clone)]
pub struct TypeSignature {
    pub name: String,
    pub matches: Expr,
    pub visual_name: String,
    pub kind: Ident,
    pub finalized: Expr,
    pub expr: Expr
}

mod kw {
    use syn::custom_keyword;

    custom_keyword!(children);
    custom_keyword!(kind);
    custom_keyword!(built);
}

impl Parse for TypeSignature {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;

        let content;
        braced!(content in input);

        let struct_kw = content.parse::<Token![struct]>()?;
        let expr = content.parse::<Expr>()?;

        content.parse::<Token![,]>()?;

        let matches_kw = content.parse::<Token![match]>()?;
        let matches: Expr = content.parse()?;

        content.parse::<Token![,]>()?;

        content.parse::<kw::kind>()?;

        let kind: Ident = content.parse()?;

        let mut finalized = matches.clone();

        if content.peek(Token![,]) && content.peek2(kw::built) {
            content.parse::<Token![,]>()?;
            content.parse::<kw::built>()?;

            finalized = content.parse()?;
        }

        Ok(TypeSignature {
            name: name.to_string(), matches, kind, visual_name: name.to_string(), finalized, expr
        })
    }
}

impl ToTokens for TypeSignature {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = &self.name;
        let kind = &self.kind;
        let matches = &self.matches;
        let matches_fn = &self.finalized;
        let expr =  &self.expr;

        tokens.append_all(
            quote! {
                (AtomStorage::atom(#name.to_string()), Arc::new(TypeSignature {
                    name: AtomStorage::atom(#name),
                    kind: DataTypeKind::#kind,
                    matches: Arc::new(#matches),
                    matches_built: Arc::new(#matches_fn),
                    underlying: Arc::new(Rw::new(#expr))
                }))
            }
        );
    }
}