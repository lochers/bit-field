use proc_macro2::{
    TokenStream,
    Span
};
use quote::{quote, ToTokens};
use syn::{
    parse2,
    braced,
    parse::{Parse, ParseStream, Result, discouraged::Speculative},
    Ident,
    token::{Colon, Comma},
    Type,
    Error,
};

use crate::bitfield::field::{
    FieldDef,
    Modifier,
    Field,
    Bits
};

use std::collections::HashSet;

pub(crate) struct BitField {
    name: Ident,
    size: Type,
    modifiers: Vec<Modifier>,
    fields: Vec<Field>,
    default: syn::LitInt
}

#[allow(dead_code)]
struct SizeDef {
    ident: Ident
}

impl Parse for SizeDef {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let ipt = input.fork();
        let ident: Result<Ident> = ipt.parse();
        match ident {
            Ok(ident) => {
                if ident == "_size" {
                    input.advance_to(&ipt);
                    return Ok(SizeDef {
                        ident
                    });
                } else {
                    return Err( Error::new_spanned(ident, "Expected _size") );
                }
            },
            Err(err) => {
                return Err( err );
            }
        }
    }
}

impl Parse for BitField {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut field_names: HashSet<Ident> = HashSet::new();
        let mut modifiers: Vec<Modifier> = Vec::new();
        let mut fields: Vec<Field> = Vec::new();
        let mut default = 0;

        let name: Ident = input.parse()?;

        let blk;
        braced!(blk in input);
        if !input.is_empty() {
            return Err( input.error("Unexpected tokens after parsing") );
        }

        // Try to parse the size field
        
        let (size, tsize) = if let Ok (_) = blk.parse::<SizeDef>() {
            let _: Colon = blk.parse()?;
            let expr: syn::Expr = blk.parse()?;
            if let syn::Expr::Lit(syn::ExprLit {lit: syn::Lit::Int(lit), ..}) = &expr {
                let num = lit.base10_parse::<usize>()?;
                if num < 8 || num > 128 || num&(num-1) != 0 {
                    return Err(Error::new_spanned(expr, "Expected a size of 8, 16, 32, 64 or 128"));
                } else {
                    let num_ident = Ident::new(&format!("u{}", num), Span::call_site());
                    if !blk.is_empty() {
                        let _: Comma = blk.parse()?;
                    }
                    (num, parse2(quote!{#num_ident})?)
                }
            } else {
                return Err(Error::new_spanned(expr, "Expected either Range or LitInt"));
            }
        } else {
            (32, parse2(quote!{u32})?)
        };

        while !blk.is_empty() {
            let fdef: FieldDef = blk.parse()?;
            if let Some(name) = field_names.get(&fdef.name) {
                let mut err = Error::new_spanned(&fdef.name, format!{"Duplicate field name"});
                err.combine(Error::new_spanned(name, format!{"First declared here"}));
                return Err(err);
            }

            field_names.insert(fdef.name.clone());

            let (modifier, field, fdefault) = fdef.constrain(size)?;
            modifiers.push(modifier);
            fields.push(field);
            default |= fdefault;
            if blk.is_empty() {
                break;
            }
            let _: Comma = blk.parse()?;
        }

        modifiers.sort_by(|a, b| a.bits.cmp(&b.bits));

        for win in modifiers.windows(2) {
            let mod1 = &win[0];
            let mod2 = &win[1];
            let upper = match mod1.bits {
                Bits::Range(_, a) => {
                    a
                },
                Bits::Single(a) => a
            };

            let lower = match mod2.bits {
                Bits::Range(a, _) => {
                    a
                },
                Bits::Single(a) => a
            };

            if upper >= lower {
                let mut err = Error::new_spanned(&mod2.field, format!{"Overlapping bit ranges"});
                err.combine(Error::new_spanned(&mod1.field, format!{"Collision here"}));
                return Err(err);
            }
        }

        Ok (BitField {
            name,
            size: tsize,
            modifiers,
            fields,
            default: syn::LitInt::new(&format!("0x{:X}", default), Span::call_site())
        })

    }
}

impl ToTokens for BitField {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = &self.name;
        let size = &self.size;
        let modifiers = &self.modifiers;
        let fields = &self.fields;
        let default = &self.default;

        tokens.extend(quote! {
            #[allow(non_camel_case_types, non_snake_case)]
            mod #name {
                use core::default::Default;
                pub use bit_field::BitField;

                #[derive(Clone, Copy)]
                pub struct Field {
                    bits: #size
                }

                impl BitField for Field {
                    type Ux = #size;

                    fn into_inner(self) -> Self::Ux {
                        self.bits
                    }
                }

                impl Default for Field {
                    fn default() -> Self {
                        Field {
                            bits: #default as <Field as BitField>::Ux
                        }
                    }
                }

                impl<'a> Field {
                    fn from(base: <Self as BitField>::Ux) -> Self {
                        Field {
                            bits: base
                        }
                    }

                    #(#fields)*
                }
                #(#modifiers)*
            }
        });
    }
}
