use convert_case::{Case, Casing};
use darling::{FromDeriveInput, FromField, FromMeta, FromVariant};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokStr;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, parse_quote};

#[derive(Debug, FromMeta, Copy, Clone, Eq, PartialEq)]
#[darling(rename_all = "snake_case")]
enum To {
    Clt,
    Srv,
}

#[derive(Debug, FromMeta)]
struct MacroArgs {
    to: To,
    repr: Option<syn::Type>,
    tag: Option<String>,
    content: Option<String>,
    #[darling(default)]
    custom: bool,
    #[darling(default)]
    enumset: bool,
}

fn wrap_attr(attr: &mut syn::Attribute) {
    let path = attr.path.clone();
    let tokens = attr.tokens.clone();

    match attr.path.get_ident().map(|i| i.to_string()).as_deref() {
        Some("mt") => {
            *attr = parse_quote! {
                #[cfg_attr(any(feature = "client", feature = "server"), #path #tokens)]
            };
        }
        Some("serde") => {
            *attr = parse_quote! {
                #[cfg_attr(feature = "serde", #path #tokens)]
            };
        }
        _ => {}
    }
}

#[proc_macro_attribute]
pub fn mt_derive(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr_args = parse_macro_input!(attr as syn::AttributeArgs);
    let mut input = parse_macro_input!(item as syn::Item);

    let args = match MacroArgs::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(e.write_errors());
        }
    };

    let (serializer, deserializer) = match args.to {
        To::Clt => ("server", "client"),
        To::Srv => ("client", "server"),
    };

    let mut out = quote! {
        #[derive(Debug)]
        #[cfg_attr(feature = "random", derive(GenerateRandom))]
        #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    };

    macro_rules! iter {
        ($t:expr, $f:expr) => {
            $t.iter_mut().for_each($f)
        };
    }

    match &mut input {
        syn::Item::Enum(e) => {
            iter!(e.attrs, wrap_attr);
            iter!(e.variants, |v| {
                iter!(v.attrs, wrap_attr);
                iter!(v.fields, |f| iter!(f.attrs, wrap_attr));
            });

            if args.enumset {
                out.extend(quote! {
                    #[derive(EnumSetType)]
                    #[enumset(serialize_as_map)]
                });

                if let Some(repr) = args.repr {
                    let repr_str = repr.to_token_stream().to_string();

                    out.extend(quote! {
                        #[enumset(repr = #repr_str)]
                    });
                } else if !args.custom {
                    panic!("missing repr for enum");
                }
            } else {
                let has_payload = e
                    .variants
                    .iter()
                    .find_map(|v| if v.fields.is_empty() { None } else { Some(()) })
                    .is_some();

                if has_payload {
                    let tag = args.tag.expect("missing tag for enum with payload");

                    out.extend(quote! {
                        #[cfg_attr(feature = "serde", serde(tag = #tag))]
                    });

                    if let Some(content) = args.content {
                        out.extend(quote! {
                            #[cfg_attr(feature = "serde", serde(content = #content))]
                        });
                    }
                } else {
                    out.extend(quote! {
                        #[derive(Copy, Eq)]
                    });
                }

                out.extend(quote! {
                    #[derive(Clone, PartialEq)]
                });

                if !args.custom {
                    out.extend(quote! {
                        #[cfg_attr(feature = #serializer, derive(MtSerialize))]
                        #[cfg_attr(feature = #deserializer, derive(MtDeserialize))]
                    });
                }

                if let Some(repr) = args.repr {
                    if repr == parse_quote! { str } {
                        out.extend(quote! {
							#[cfg_attr(any(feature = "client", feature = "server"), mt(string_repr))]
						});
                    } else {
                        out.extend(quote! {
                            #[repr(#repr)]
                        });
                    }
                } else if !args.custom {
                    panic!("missing repr for enum");
                }
            }

            out.extend(quote! {
                #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
            });
        }
        syn::Item::Struct(s) => {
            iter!(s.attrs, wrap_attr);
            iter!(s.fields, |f| iter!(f.attrs, wrap_attr));

            out.extend(quote! {
                #[derive(Clone, PartialEq)]
            });

            if !args.custom {
                out.extend(quote! {
                    #[cfg_attr(feature = #serializer, derive(MtSerialize))]
                    #[cfg_attr(feature = #deserializer, derive(MtDeserialize))]
                });
            }
        }
        _ => panic!("only enum and struct supported"),
    }

    out.extend(input.to_token_stream());
    out.into()
}

#[derive(Debug, Default, FromDeriveInput, FromVariant, FromField)]
#[darling(attributes(mt))]
#[darling(default)]
struct MtArgs {
    #[darling(multiple)]
    const_before: Vec<syn::Expr>, // must implement MtSerialize + MtDeserialize + PartialEq
    #[darling(multiple)]
    const_after: Vec<syn::Expr>, // must implement MtSerialize + MtDeserialize + PartialEq
    size: Option<syn::Type>, // must implement MtCfg
    len: Option<syn::Type>,  // must implement MtCfg
    default: bool,           // type must implement Default
    string_repr: bool,       // for enums
    zlib: bool,
    zstd: bool,
    map_ser: Option<syn::Expr>,
    map_des: Option<syn::Expr>,
    multiplier: Option<syn::Expr>,
    typename: Option<syn::Ident>, // remote derive
    bounds: Option<syn::WhereClause>,
}

type Fields<'a> = Vec<(TokStr, &'a syn::Field)>;

fn get_fields(fields: &syn::Fields, ident: impl Fn(TokStr) -> TokStr) -> Fields {
    match fields {
        syn::Fields::Named(fs) => fs
            .named
            .iter()
            .map(|f| (ident(f.ident.as_ref().unwrap().to_token_stream()), f))
            .collect(),
        syn::Fields::Unnamed(fs) => fs
            .unnamed
            .iter()
            .enumerate()
            .map(|(i, f)| (ident(syn::Index::from(i).to_token_stream()), f))
            .collect(),
        syn::Fields::Unit => Vec::new(),
    }
}

fn serialize_args(args: &MtArgs, code: &mut TokStr) {
    macro_rules! impl_compress {
        ($create:expr) => {
            *code = quote! {
                let mut __writer = {
                    let mut __stream = $create;
                    let __writer = &mut __stream;
                    #code
                    __stream.finish()?
                };
            };
        };
    }

    if args.zlib {
        impl_compress!(mt_ser::flate2::write::ZlibEncoder::new(
            __writer,
            mt_ser::flate2::Compression::default()
        ));
    }

    if args.zstd {
        impl_compress!(mt_ser::zstd::stream::write::Encoder::new(__writer, 0)?);
    }

    if let Some(size) = &args.size {
        *code = quote! {
            mt_ser::MtSerialize::mt_serialize::<#size>(&{
                let mut __buf = Vec::new();
                let __writer = &mut __buf;
                #code
                __buf
            }, __writer)?;
        };
    }

    for x in args.const_before.iter().rev() {
        *code = quote! {
            #x.mt_serialize::<mt_ser::DefCfg>(__writer)?;
            #code
        }
    }

    for x in args.const_after.iter() {
        *code = quote! {
            #code
            #x.mt_serialize::<mt_ser::DefCfg>(__writer)?;
        }
    }
}

fn deserialize_args(args: &MtArgs, code: &mut TokStr) {
    macro_rules! impl_compress {
        ($create:expr) => {
            *code = quote! {
                {
                    let mut __owned_reader = $create;
                    let __reader = &mut __owned_reader;

                    #code
                }
            }
        };
    }

    if args.zlib {
        impl_compress!(mt_ser::flate2::read::ZlibDecoder::new(mt_ser::WrapRead(
            __reader
        )));
    }

    if args.zstd {
        impl_compress!(mt_ser::zstd::stream::read::Decoder::new(mt_ser::WrapRead(
            __reader
        ))?);
    }

    if let Some(size) = &args.size {
        *code = quote! {
            {
                let __size = #size::mt_deserialize::<DefCfg>(__reader)? as u64;
                let mut __owned_reader = std::io::Read::take(
                    mt_ser::WrapRead(__reader),
                    __size,
                );
                let __reader = &mut __owned_reader;

                let __result = { #code };

                std::io::Read::read_to_end(
                    __reader,
                    &mut Vec::with_capacity(__reader.limit() as usize),
                )?;

                __result
            }
        };
    }

    let impl_const = |want: &syn::Expr| {
        quote! {
            {
                fn eq_same_type<T: PartialEq<T>>(a: &T, b: &T) -> bool {
                    a == b
                }

                let want = #want;
                let got = mt_ser::MtDeserialize::mt_deserialize::<DefCfg>(__reader)?;

                if !eq_same_type(&want, &got) {
                    return Err(mt_ser::DeserializeError::InvalidConst(
                        Box::new(want), Box::new(got)
                    ));
                }
            }
        }
    };

    for want in args.const_before.iter().rev() {
        let imp = impl_const(want);
        *code = quote! {
            {
                #imp
                #code
            }
        };
    }

    for want in args.const_after.iter() {
        let imp = impl_const(want);
        *code = quote! {
            {
                let __result = #code;
                #imp
                __result
            }
        };
    }
}

fn serialize_fields(fields: &Fields) -> TokStr {
    fields
        .iter()
        .map(|(ident, field)| {
            let args = MtArgs::from_field(field).unwrap();
            let def = parse_quote! { mt_ser::DefCfg };
            let len = args.len.as_ref().unwrap_or(&def);

            let mut code = quote! { #ident };

            if let Some(multiplier) = &args.multiplier {
                code = quote! {
                    &((#code) * (#multiplier))
                };
            }

            if let Some(map) = &args.map_ser {
                code = quote! {
                    {
                        fn call_ser_result<I, O>(
                            f: impl FnOnce(I) -> Result<O, mt_ser::SerializeError>,
                            i: I
                        ) -> Result<O, mt_ser::SerializeError> {
                            f(i)
                        }

                        &call_ser_result(#map, #code)?
                    }
                };
            }

            code = quote! { mt_ser::MtSerialize::mt_serialize::<#len>(#code, __writer)?; };

            serialize_args(&args, &mut code);

            code
        })
        .collect()
}

fn deserialize_fields(fields: &Fields) -> TokStr {
    fields
        .iter()
        .map(|(ident, field)| {
            let args = MtArgs::from_field(field).unwrap();

            let def = parse_quote! { mt_ser::DefCfg };
            let len = args.len.as_ref().unwrap_or(&def);
            let mut code = quote! { mt_ser::MtDeserialize::mt_deserialize::<#len>(__reader) };

            if args.default {
                code = quote! {
                    mt_ser::OrDefault::or_default(#code)
                };
            }

            code = quote! {
                (#code)?
            };

            deserialize_args(&args, &mut code);

            if let Some(map) = &args.map_des {
                code = quote! {
                    {
                        fn call_des_result<I, O>(
                            f: impl FnOnce(I) -> Result<O, mt_ser::DeserializeError>,
                            i: I
                        ) -> Result<O, mt_ser::DeserializeError> {
                            f(i)
                        }

                        call_des_result(#map, #code)?
                    }
                };
            }

            if let Some(multiplier) = &args.multiplier {
                code = quote! {
                    {
                        fn div_same_type<D, T: std::ops::Div<D, Output = T>>(a: T, b: D) -> T {
                            a / b
                        }

                        div_same_type(#code, #multiplier)
                    }
                }
            }

            quote! {
                let #ident = #code;
            }
        })
        .collect()
}

fn get_fields_struct(input: &syn::Fields) -> (Fields, TokStr) {
    let ident_fn = match input {
        syn::Fields::Unnamed(_) => |f| {
            quote! {
                mt_ser::paste::paste! { [<field_ #f>] }
            }
        },
        _ => |f| quote! { #f },
    };

    let fields = get_fields(input, ident_fn);
    let fields_comma: TokStr = fields
        .iter()
        .rfold(TokStr::new(), |after, (ident, _)| quote! { #ident, #after });

    let fields_struct = match input {
        syn::Fields::Named(_) => quote! { { #fields_comma } },
        syn::Fields::Unnamed(_) => quote! { ( #fields_comma ) },
        syn::Fields::Unit => TokStr::new(),
    };

    (fields, fields_struct)
}

fn get_repr(input: &syn::DeriveInput, args: &MtArgs) -> syn::Type {
    if args.string_repr {
        parse_quote! { &str }
    } else {
        input
            .attrs
            .iter()
            .find(|a| a.path.is_ident("repr"))
            .expect("missing repr")
            .parse_args()
            .expect("invalid repr")
    }
}

fn iter_variants(e: &syn::DataEnum, args: &MtArgs, mut f: impl FnMut(&syn::Variant, &syn::Expr)) {
    let mut discr = parse_quote! { 0 };

    for v in e.variants.iter() {
        discr = if args.string_repr {
            let lit = v.ident.to_string().to_case(Case::Snake);
            parse_quote! { #lit }
        } else {
            v.discriminant.clone().map(|x| x.1).unwrap_or(discr)
        };

        f(v, &discr);

        discr = parse_quote! { 1 + #discr };
    }
}

fn make_impl(
    traitname: TokStr,
    input: &syn::DeriveInput,
    typename: &syn::Ident,
    args: &MtArgs,
    code: TokStr,
) -> TokenStream {
    let generics = &input.generics;
    let bounds = args.bounds.clone().or_else(|| {
        if generics.params.is_empty() {
            None
        } else {
            Some(
                syn::parse(
                    generics
                        .params
                        .iter()
                        .rfold(quote! { where }, |before, t| match t {
                            syn::GenericParam::Type(x) => quote! { #before #x: #traitname, },
                            _ => before,
                        })
                        .into(),
                )
                .expect("invalid where clause"),
            )
        }
    });

    quote! {
        #[automatically_derived]
        impl #generics #traitname for #typename #generics #bounds { #code }
    }
    .into()
}

#[proc_macro_derive(MtSerialize, attributes(mt))]
pub fn derive_serialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let args = MtArgs::from_derive_input(&input).unwrap();
    let typename = args.typename.as_ref().unwrap_or(&input.ident);

    let mut code = match &input.data {
        syn::Data::Enum(e) => {
            let repr = get_repr(&input, &args);
            let mut variants = TokStr::new();

            iter_variants(e, &args, |v, discr| {
                let args = MtArgs::from_variant(v).unwrap();

                let (fields, fields_struct) = get_fields_struct(&v.fields);

                let mut code = serialize_fields(&fields);
                serialize_args(&args, &mut code);

                let ident = &v.ident;

                variants.extend(quote! {
					#typename::#ident #fields_struct => {
						mt_ser::MtSerialize::mt_serialize::<mt_ser::DefCfg>(&((#discr) as #repr), __writer)?;
						#code
					}
				});
            });

            quote! {
                match self {
                    #variants
                }
            }
        }
        syn::Data::Struct(s) => serialize_fields(&get_fields(&s.fields, |f| quote! { &self.#f })),
        _ => {
            panic!("only enum and struct supported");
        }
    };

    serialize_args(&args, &mut code);

    make_impl(
        quote! { mt_ser::MtSerialize },
        &input,
        typename,
        &args,
        quote! {
            fn mt_serialize<C: mt_ser::MtCfg>(&self, __writer: &mut impl std::io::Write) -> Result<(), mt_ser::SerializeError> {
                #code

                Ok(())
            }
        },
    )
}

#[proc_macro_derive(MtDeserialize, attributes(mt))]
pub fn derive_deserialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let args = MtArgs::from_derive_input(&input).unwrap();
    let typename = args.typename.as_ref().unwrap_or(&input.ident);

    let mut code = match &input.data {
        syn::Data::Enum(e) => {
            let repr = get_repr(&input, &args);

            let mut consts = TokStr::new();
            let mut arms = TokStr::new();

            iter_variants(e, &args, |v, discr| {
                let args = MtArgs::from_variant(v).unwrap();

                let ident = &v.ident;
                let (fields, fields_struct) = get_fields_struct(&v.fields);
                let mut code = deserialize_fields(&fields);
                code = quote! {
                    #code
                    Ok(Self::#ident #fields_struct)
                };

                deserialize_args(&args, &mut code);

                consts.extend(quote! {
                    const #ident: #repr = #discr;
                });

                arms.extend(quote! {
                    #ident => { #code }
                });
            });

            let type_str = typename.to_string();
            let discr_match = if args.string_repr {
                quote! {
                    let __discr = String::mt_deserialize::<DefCfg>(__reader)?;
                    match __discr.as_str()
                }
            } else {
                quote! {
                    let __discr = mt_ser::MtDeserialize::mt_deserialize::<DefCfg>(__reader)?;
                    match __discr
                }
            };

            quote! {
                #consts

                #discr_match {
                    #arms
                    _ => Err(mt_ser::DeserializeError::InvalidEnum(#type_str, Box::new(__discr)))
                }
            }
        }
        syn::Data::Struct(s) => {
            let (fields, fields_struct) = get_fields_struct(&s.fields);
            let code = deserialize_fields(&fields);

            quote! {
                #code
                Ok(Self #fields_struct)
            }
        }
        _ => {
            panic!("only enum and struct supported");
        }
    };

    deserialize_args(&args, &mut code);

    make_impl(
        quote! { mt_ser::MtDeserialize },
        &input,
        typename,
        &args,
        quote! {
            #[allow(non_upper_case_globals)]
            fn mt_deserialize<C: mt_ser::MtCfg>(__reader: &mut impl std::io::Read) -> Result<Self, mt_ser::DeserializeError> {
                #code
            }
        },
    )
}
