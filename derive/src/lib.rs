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

#[derive(Debug, Default, FromDeriveInput, FromVariant, FromField)]
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
	utf16: bool,
	zlib: bool,
	zstd: bool, // TODO
	default: bool,
}

fn get_cfg(args: &MtArgs) -> syn::Type {
	let mut ty: syn::Type = parse_quote! { mt_ser::DefCfg  };

	if args.len0 {
		ty = parse_quote! { () };
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
		ty = parse_quote! { mt_ser::Utf16<#ty> };
	}

	ty
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
			.map(|(i, f)| (ident(i.to_string().to_token_stream()), f))
			.collect(),
		syn::Fields::Unit => Vec::new(),
	}
}

fn serialize_args(res: darling::Result<MtArgs>, body: impl FnOnce(&MtArgs) -> TokStr) -> TokStr {
	match res {
		Ok(args) => {
			let mut code = TokStr::new();

			macro_rules! impl_const {
				($name:ident) => {
					if let Some(x) = args.$name {
						code.extend(quote! {
							#x.mt_serialize::<mt_ser::DefCfg>(__writer)?;
						});
					}
				};
			}

			impl_const!(const8);
			impl_const!(const16);
			impl_const!(const32);
			impl_const!(const64);

			code.extend(body(&args));

			if args.zlib {
				code = quote! {
					let mut __writer = {
						let mut __stream = mt_ser::flate2::write::ZlibEncoder::new(
							__writer,
							mt_ser::flate2::Compression::default(),
						);
						let __writer = &mut __stream;
						#code
						__stream.finish()?
					};
				};
			}

			macro_rules! impl_size {
				($name:ident, $T:ty) => {
					if args.$name {
						code = quote! {
								mt_ser::MtSerialize::mt_serialize::<$T>(&{
									let mut __buf = Vec::new();
									let __writer = &mut __buf;
									#code
									__buf
								}, __writer)?;
						};
					}
				};
			}

			impl_size!(size8, u8);
			impl_size!(size16, u16);
			impl_size!(size32, u32);
			impl_size!(size64, u64);

			code
		}
		Err(e) => return e.write_errors(),
	}
}

fn serialize_fields(fields: &Fields) -> TokStr {
	fields
		.iter()
		.map(|(ident, field)| {
			serialize_args(MtArgs::from_field(field), |args| {
				let cfg = get_cfg(args);
				quote! { mt_ser::MtSerialize::mt_serialize::<#cfg>(#ident, __writer)?; }
			})
		})
		.collect()
}

#[proc_macro_derive(MtSerialize, attributes(mt))]
pub fn derive_serialize(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as syn::DeriveInput);
	let typename = &input.ident;

	let code = serialize_args(MtArgs::from_derive_input(&input), |_| match &input.data {
		syn::Data::Enum(e) => {
			let repr: syn::Type = input
				.attrs
				.iter()
				.find(|a| a.path.is_ident("repr"))
				.expect("missing repr")
				.parse_args()
				.expect("invalid repr");

			let variants: TokStr = e.variants
				.iter()
				.fold((parse_quote! { 0 }, TokStr::new()), |(discr, before), v| {
					let discr = v.discriminant.clone().map(|x| x.1).unwrap_or(discr);

					let ident_fn = match &v.fields {
						syn::Fields::Unnamed(_) => |f| quote! {
							mt_ser::paste::paste! { [<field_ #f>] }
						},
						_ => |f| quote! { #f },
					};

					let fields = get_fields(&v.fields, ident_fn);
					let fields_comma: TokStr = fields.iter()
						.rfold(TokStr::new(), |after, (ident, _)| quote! { #ident, #after });

					let destruct = match &v.fields {
						syn::Fields::Named(_) => quote! { { #fields_comma } },
						syn::Fields::Unnamed(_) => quote! { ( #fields_comma ) },
						syn::Fields::Unit => TokStr::new(),
					};

					let code = serialize_args(MtArgs::from_variant(v), |_|
						serialize_fields(&fields));
					let variant = &v.ident;

					(
						parse_quote! { 1 + #discr },
						quote! {
							#before
							#typename::#variant #destruct => {
								mt_ser::MtSerialize::mt_serialize::<mt_ser::DefCfg>(&((#discr) as #repr), __writer)?;
								#code
							}
						}
					)
				}).1;

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
	});

	quote! {
		#[automatically_derived]
		impl mt_ser::MtSerialize for #typename {
			fn mt_serialize<C: mt_ser::MtCfg>(&self, __writer: &mut impl std::io::Write) -> Result<(), mt_ser::SerializeError> {
				#code

				Ok(())
			}
		}
	}.into()
}

#[proc_macro_derive(MtDeserialize, attributes(mt))]
pub fn derive_deserialize(input: TokenStream) -> TokenStream {
	let syn::DeriveInput {
		ident: typename, ..
	} = parse_macro_input!(input);
	quote! {
		#[automatically_derived]
		impl mt_ser::MtDeserialize for #typename {
			fn mt_deserialize<C: mt_ser::MtCfg>(__reader: &mut impl std::io::Read) -> Result<Self, mt_ser::DeserializeError> {
				Err(mt_ser::DeserializeError::Unimplemented)
			}
		}
	}.into()
}
