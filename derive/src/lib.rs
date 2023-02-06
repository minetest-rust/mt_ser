use darling::FromMeta;
use proc_macro::{self, TokenStream};
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
						#[derive(Clone)]
						#[cfg_attr(feature = "serde", serde(tag = #tag))]
					});

					if let Some(content) = args.content {
						out.extend(quote! {
							#[cfg_attr(feature = "serde", serde(content = #content))]
						});
					}
				} else {
					out.extend(quote! {
						#[derive(Copy, Clone, PartialEq, Eq)]
					});
				}

				out.extend(quote! {
					#[repr(#repr)]
					#[cfg_attr(feature = #serializer, derive(MtSerialize))]
					#[cfg_attr(feature = #deserializer, derive(MtDeserialize))]
				});
			}

			out.extend(quote! {
				#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
			});
		}
		syn::Item::Struct(s) => {
			iter!(s.attrs, wrap_attr);
			iter!(s.fields, |f| iter!(f.attrs, wrap_attr));

			out.extend(quote! {
				#[derive(Clone)]
				#[cfg_attr(feature = #serializer, derive(MtSerialize))]
				#[cfg_attr(feature = #deserializer, derive(MtDeserialize))]
			});
		}
		_ => panic!("only enum and struct supported"),
	}

	out.extend(input.to_token_stream());
	out.into()
}

#[proc_macro_derive(MtSerialize, attributes(mt))]
pub fn derive_serialize(input: TokenStream) -> TokenStream {
	let syn::DeriveInput { ident, .. } = parse_macro_input!(input);
	let output = quote! {
		impl MtSerialize for #ident {
			fn mt_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<(), mt_data::SerializeError> {
				Err(mt_data::SerializeError::Unimplemented)
			}
		}
	};
	output.into()
}

#[proc_macro_derive(MtDeserialize, attributes(mt))]
pub fn derive_deserialize(input: TokenStream) -> TokenStream {
	let syn::DeriveInput { ident, .. } = parse_macro_input!(input);
	quote! {
		impl MtDeserialize for #ident {
			fn mt_deserialize<R: std::io::Read>(reader: &mut R) -> Result<Self, mt_data::DeserializeError> {
				Err(mt_data::DeserializeError::Unimplemented)
			}
		}
	}.into()
}
