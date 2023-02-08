use darling::{FromField, FromMeta};
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
    match attr.path.get_ident().map(|i| i.to_string()).as_deref() {
        Some("mt") => {
            let path = attr.path.clone();
            let tokens = attr.tokens.clone();

            *attr = parse_quote! {
                #[cfg_attr(any(feature = "client", feature = "server"), #path #tokens)]
            };
        }
        Some("serde") => {
            let path = attr.path.clone();
            let tokens = attr.tokens.clone();

            *attr = parse_quote! {
                #[cfg_attr(feature = "serde", #path #tokens)]
            };
        }
        _ => {}
    }
}

#[proc_macro_attribute]
pub fn mt_derive(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item2 = item.clone();

    let attr_args = parse_macro_input!(attr as syn::AttributeArgs);
    let mut input = parse_macro_input!(item2 as syn::Item);

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

            let repr = args.repr.expect("missing repr for enum");

            if args.enumset {
                let repr_str = repr.to_token_stream().to_string();

                out.extend(quote! {
                    #[derive(EnumSetType)]
                    #[enumset(repr = #repr_str, serialize_as_map)]
                })
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
                    #[repr(#repr)]
                    #[derive(Clone, PartialEq)]
                });

                if !args.custom {
                    out.extend(quote! {
                        #[cfg_attr(feature = #serializer, derive(MtSerialize))]
                        #[cfg_attr(feature = #deserializer, derive(MtDeserialize))]
                    });
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

#[derive(Debug, Default, FromField)]
#[darling(attributes(mt))]
#[darling(default)]
struct MtArgs {
    const8: Option<u8>,
    const16: Option<u16>,
    const32: Option<u32>,
    const64: Option<u64>,
    size8: bool,
    size16: bool,
    size32: bool,
    size64: bool,
    len0: bool,
    len8: bool,
    len16: bool,
    len32: bool,
    len64: bool,
    utf16: bool, // TODO
    zlib: bool,
    default: bool,
}

fn get_cfg(args: &MtArgs) -> syn::Type {
    let mut ty: syn::Type = parse_quote! { mt_data::DefaultCfg  };

    if args.len0 {
        ty = parse_quote! { mt_data::NoLen };
    }

    macro_rules! impl_len {
        ($name:ident, $T:ty) => {
            if args.$name {
                ty = parse_quote! { $T  };
            }
        };
    }

    impl_len!(len8, u8);
    impl_len!(len16, u16);
    impl_len!(len32, u32);
    impl_len!(len64, u64);

    if args.utf16 {
        ty = parse_quote! { mt_data::Utf16<#ty> };
    }

    ty
}

/*
fn is_ident(path: &syn::Path, ident: &str) -> bool {
    matches!(path.segments.first().map(|p| &p.ident), Some(idt) if idt == ident)
}

fn get_type_generics<const N: usize>(path: &syn::Path) -> Option<[&syn::Type; N]> {
    use syn::{AngleBracketedGenericArguments as Args, PathArguments::AngleBracketed};

    path.segments
        .first()
        .map(|seg| match &seg.arguments {
            AngleBracketed(Args { args, .. }) => args
                .iter()
                .flat_map(|arg| match arg {
                    syn::GenericArgument::Type(t) => Some(t),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .try_into()
                .ok(),
            _ => None,
        })
        .flatten()
}
*/

fn serialize_fields(fields: &syn::Fields) -> TokStr {
    let mut code: TokStr = (match fields {
		syn::Fields::Named(fs) => fs
			.named
			.iter()
			.map(|f| (f.ident.as_ref().unwrap().to_token_stream(), f))
			.collect(),
		syn::Fields::Unnamed(fs) => fs
			.unnamed
			.iter()
			.enumerate()
			.map(|(i, f)| (i.to_token_stream(), f))
			.collect(),
		syn::Fields::Unit => Vec::new(),
	}).into_iter().map(|(ident, field)| {
		let args = match MtArgs::from_field(field) {
			Ok(v) => v,
			Err(e) => return e.write_errors(),
		};

		let mut code = TokStr::new();

		macro_rules! impl_const {
			($name:ident) => {
				if let Some(x) = args.$name {
					code.extend(quote! {
						#x.mt_serialize::<mt_data::DefaultCfg>(writer)?;
					});
				}
			};
		}

		impl_const!(const8);
		impl_const!(const16);
		impl_const!(const32);
		impl_const!(const64);

		let cfg = get_cfg(&args);
		code.extend(quote! {
			mt_data::MtSerialize::mt_serialize::<#cfg>(&self.#ident, writer)?;
		});

		if args.zlib {
			code = quote! {
				let mut writer = {
					let mut writer = mt_data::flate2::write::ZlibEncoder(writer, flate2::Compression::default());
					#code
					writer.finish()?
				};
			};
		}

		macro_rules! impl_size {
			($name:ident, $T:ty) => {
				if args.$name {
					code = quote! {
						{
							let buf = {
								let mut writer = Vec::new();
								#code
								writer
							};

							TryInto::<$T>::try_into(buf.len())?.mt_serialize::<mt_data::DefaultCfg>();
						}
					};
				}
			};
		}

		impl_size!(size8, u8);
		impl_size!(size16, u16);
		impl_size!(size32, u32);
		impl_size!(size64, u64);

		code
	}).collect();

    code.extend(quote! {
        Ok(())
    });

    code
}

#[proc_macro_derive(MtSerialize, attributes(mt))]
pub fn derive_serialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let ident = &input.ident;

    let code = match &input.data {
        syn::Data::Enum(_e) => quote! {
            Err(mt_data::SerializeError::Unimplemented)
        },
        syn::Data::Struct(s) => serialize_fields(&s.fields),
        _ => {
            panic!("only enum and struct supported");
        }
    };

    quote! {
		#[automatically_derived]
		impl mt_data::MtSerialize for #ident {
			fn mt_serialize<C: MtCfg>(&self, writer: &mut impl std::io::Write) -> Result<(), mt_data::SerializeError> {
				#code
			}
		}
	}.into()
}

#[proc_macro_derive(MtDeserialize, attributes(mt))]
pub fn derive_deserialize(input: TokenStream) -> TokenStream {
    let syn::DeriveInput { ident, .. } = parse_macro_input!(input);
    quote! {
		#[automatically_derived]
		impl mt_data::MtDeserialize for #ident {
			fn mt_deserialize<C: MtCfg>(reader: &mut impl std::io::Read) -> Result<Self, mt_data::DeserializeError> {
				Err(mt_data::DeserializeError::Unimplemented)
			}
		}
	}.into()
}
