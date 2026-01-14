use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{braced, custom_keyword, Expr, Token, Type};
use syn::parse::{Parse, ParseStream};

#[derive(Clone)]
pub struct TypeSignature {
    pub name: String,
    pub matches: Expr,
    pub children: Vec<TypeSignature>,
    pub visual_name: String,
    pub kind: Ident,
    pub finalized: Expr
}

mod kw {
    use syn::custom_keyword;

    custom_keyword!(children);
    custom_keyword!(kind);
    custom_keyword!(finalized);
}

impl Parse for TypeSignature {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;

        let content;
        braced!(content in input);

        let matches_kw = content.parse::<Token![match]>()?;
        let matches: Expr = content.parse()?;

        content.parse::<Token![,]>()?;

        content.parse::<kw::kind>()?;

        let kind: Ident = content.parse()?;

        let mut finalized = matches.clone();

        if content.peek(Token![,]) && content.peek2(kw::finalized) {
            content.parse::<Token![,]>()?;
            content.parse::<kw::finalized>()?;

            finalized = content.parse()?;
        }

        let mut children: Vec<TypeSignature> = Vec::new();

        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
            content.parse::<kw::children>()?;

            let cntnt;
            braced!(cntnt in content);

            let children_def = cntnt.parse_terminated(TypeSignature::parse, Token![,])?;

            children = children_def.into_iter().collect();
        }

        Ok(TypeSignature {
            name: name.to_string(), matches, children, kind, visual_name: name.to_string(), finalized
        })
    }
}

impl ToTokens for TypeSignature {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = &self.name;
        let kind = &self.kind;
        let matches = &self.matches;
        let vis_name = &self.visual_name;
        let matches_fn = &self.finalized;


        let mut c = self.children.clone();
        let children: Vec<&mut TypeSignature> = c.iter_mut().map(|x| { x.visual_name = format!("{name}.{}", x.visual_name); x }).collect();

        tokens.append_all(
            quote! {
                (AtomStorage::atom(#name.to_string()), Arc::new(DataTypeSignature {
                    name: #name.to_string(),
                    kind: DataTypeKind::#kind,
                    matches: Arc::new(#matches),
                    visual_name: #vis_name.to_string(),
                    matches_finalized: Arc::new(#matches_fn),
                    children: HashMap::from([
                        #(#children),*
                    ])
                }))
            }
        );
    }
}