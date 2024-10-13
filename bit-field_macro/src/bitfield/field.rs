use proc_macro2:: {
    TokenStream,
    Span
};
use quote::{quote, ToTokens};
use syn::{
    Token,
    parse2,
    parse::{Parse, ParseStream, Result},
    Ident,
    token::Colon,
    Type,
    LitInt,
    Error,
};

pub enum UCBits {
    Range(UCRange),
    Single(LitInt)
}

pub enum Bits {
    Range(usize, usize),
    Single(usize)
}

pub enum UCRange {
    Open(LitInt, LitInt),
    Closed(LitInt, LitInt)
}

pub(crate) struct Field {
    writer: Ident, 
    writer_s: Ident,
    reader: Ident,
    reader_s: Ident
}

pub(crate) struct FieldDef {
    pub(crate) name: Ident,
    pub(crate) bits: UCBits,
    pub(crate) default: Option<LitInt>,
}

impl Parse for FieldDef {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name: Ident = input.parse()?;
        let _: Colon = input.parse()?;

        let start: LitInt = input.parse()?;
        let bits = if let Ok(_) = input.parse::<Token![..]>() {
            if let Ok(_) = input.parse::<Token![=]>() {
                UCBits::Range(UCRange::Closed(start, input.parse()?)) 
            } else {
                UCBits::Range(UCRange::Open(start, input.parse()?)) 
            }
            
        } else {
            UCBits::Single(start)
        };

        let default: Option<LitInt> = if let Ok(_) = input.parse::<Token![=]>() {
            Some( input.parse()? )
        } else {
            None
        };

        Ok (FieldDef {
            name,
            bits,
            default
        })

    }
}

impl FieldDef {
    pub fn constrain(self, size: usize) -> Result<(Modifier, Field, usize)>{
        let mr_name = Ident::new(&format!("{}_R", self.name.to_string().to_uppercase()), Span::call_site());
        let mw_name = Ident::new(&format!("{}_W", self.name.to_string().to_uppercase()), Span::call_site());
        let fr_name = Ident::new(&format!("r_{}", self.name.to_string().to_lowercase()), Span::call_site());
        let fw_name = Ident::new(&format!("w_{}", self.name.to_string().to_lowercase()), Span::call_site());
        
        let (bits, size, start) = match self.bits {
            UCBits::Range(ucrng) => {
                let (start, end) = match ucrng {
                    UCRange::Closed(start, end) => {
                        let start = start.base10_parse::<usize>()?;
                        let end = end.base10_parse::<usize>()?;

                        if start >= end {
                            let err = Error::new_spanned(start, "Start must be strictly less than end.");
                            return Err(err);
                        }

                        if end >= size {
                            let err = Error::new_spanned(start, "End is out of bounds.");
                            return Err(err);
                        }
                        (start, end)
                    },
                    UCRange::Open(start, end) => {
                        let start = start.base10_parse::<usize>()?;
                        let end = end.base10_parse::<usize>()? - 1;

                        if start >= end {
                            let err = Error::new_spanned(start, "Start must be strictly less than end.");
                            return Err(err);
                        }

                        if end >= size {
                            let err = Error::new_spanned(start, "End is out of bounds.");
                            return Err(err);
                        }
                        (start, end)
                    }
                };

                (Bits::Range(start, end), (end - start) as u32, start)
            },
            UCBits::Single(lit) => {
                let a = lit.base10_parse::<usize>()?;
                if a >= size {
                    let err = Error::new_spanned(lit, "Field out of bounds");
                    return Err(err);
                }
                (Bits::Single(a), 1, a)
            }
        };

        let default = if let Some (lit) = self.default {
            let num = lit.base10_parse::<usize>()?;
            let num_size = std::mem::size_of::<usize>() as u32 * 8 - num.leading_zeros();
            if num_size > size {
                let err = Error::new_spanned(lit, "Default too large to fit in field");
                return Err(err)
            }
            num
        } else {
            0
        };


        Ok((Modifier {
            field: self.name,
            writer: mw_name.clone(),
            reader: mr_name.clone(),
            bits
        },
        Field {
            writer: fw_name,
            writer_s: mw_name,
            reader: fr_name,
            reader_s: mr_name
        },
        default << start))
    }
}

pub struct Modifier {
    pub(crate) field: Ident,
    pub(crate) writer: Ident,
    pub(crate) reader: Ident,
    pub(crate) bits: Bits
}

impl Ord for Bits {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl PartialOrd for Bits {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let a1 = match self {
            Bits::Range(a, _) => {
                a
            },
            Bits::Single(a) => {
                a
            }
        };

        let a2 = match other {
            Bits::Range(a, _) => {
                a
            },
            Bits::Single(a) => {
                a
            }
        };
        Some(a1.cmp(a2))
    }
}

impl Eq for Bits {}

impl PartialEq for Bits {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Bits::Range(a1, b1) => {
                if let Bits::Range(a2, b2) = other {
                    a1 == a2 && b1 == b2
                } else {
                    false
                }
            },
            Bits::Single(a1) => {
                if let Bits::Single(a2) = other {
                    a1 == a2
                } else {
                    false
                }
            }
        }
    }
}



fn get_holding_type(n_bits: usize) -> Type {
    parse2(match n_bits {
        1..=8 => quote!{u8},
        9..=16 => quote!{u16},
        17..=32 => quote!{u32},
        31..=64 => quote!{u64},
        65..=128 => quote!{u128},
        _ => unreachable!()
    }).unwrap()
//
//    Type::Path(
//        TypePath {
//            qself: None,
//            path: Path {
//                leading_colon: None,
//                segments: vec![
//                    PathSegment {
//                        ident: Ident::new(&ty, Span::call_site()),
//                        arguments: syn::PathArguments::None
//                    }
//                ].into_iter().collect()
//            }
//        }
//    )
}

impl ToTokens for Field {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let writer = &self.writer;
        let writer_s = &self.writer_s;
        let reader = &self.reader;
        let reader_s = &self.reader_s;

        tokens.extend(quote! {
            pub fn #writer(&'a mut self) -> #writer_s<'a> {
                #writer_s {
                    f: self
                }
            }

            pub fn #reader(&'a self) -> #reader_s<'a> {
                #reader_s {
                    f: self
                }
            }
        });
    }
}

impl ToTokens for Modifier {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let writer = &self.writer;
        let reader = &self.reader;

        match self.bits {
            Bits::Range(s, e) => {
                let dist = e - s;
                let mask = LitInt::new(&format!("0x{:X}", (1 << dist) - 1), Span::call_site());
                let enc = get_holding_type(dist);
                tokens.extend(quote! {
                    pub struct #writer<'a> {
                        f: &'a mut Field
                    }
                    
                    impl<'a> #writer<'a> {
                        pub fn bits(self, bits: #enc) -> &'a mut Field {
                            let flip = ((bits as <Field as BitField>::Ux) & #mask) << #s;
                            self.f.bits = (self.f.bits & !(#mask<<#s)) | flip;
                            self.f
                        }
                    }

                    pub struct #reader<'a> {
                        f: &'a Field
                    }
                    
                    impl<'a> #reader<'a> {
                        pub fn get_bits(self) ->  #enc {
                            ((self.f.bits & (#mask<<#s)) >> #s) as #enc
                        }
                    }
                });
            },
            Bits::Single(s) => {
                tokens.extend(quote! {
                    pub struct #writer<'a> {
                        f: &'a mut Field
                    }
                    
                    impl<'a> #writer<'a> {
                        pub fn bit(self, b: bool) -> &'a mut Field {
                            let flip = (b as <Field as BitField>::Ux) << #s;
                            self.f.bits = (self.f.bits & !(1<<#s)) | flip;
                            self.f
                        }

                        pub fn clear_bit(self) -> &'a mut Field {
                            self.f.bits = self.f.bits & !(1<<#s);
                            self.f
                        }

                        pub fn set_bit(self) -> &'a mut Field {
                            self.f.bits = (self.f.bits & !(1<<#s)) | (1<<#s);
                            self.f
                        }
                    }

                    pub struct #reader<'a> {
                        f: &'a Field
                    }

                    impl<'a> #reader<'a> {
                        pub fn is_bit_set(self) -> bool {
                            self.f.bits & (1<<#s) != 0 
                        }

                        pub fn is_bit_clear(self) -> bool {
                            self.f.bits & (1<<#s) == 0 
                        }
                        
                        pub fn get_bit(self) -> u8 {
                            ((self.f.bits & (1<<#s)) >> #s) as u8
                        }
                    }
                });
            }
        }
    }
}
