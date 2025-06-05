// Number enum

use quote::quote;
use proc_macro2::TokenStream;
use syn::{parse_macro_input, parse_quote, Attribute, DataEnum, DeriveInput, Ident, Meta};

macro_rules! rt_error {
    ($span:expr, $msg:expr) => {
        return proc_macro::TokenStream::from(
            syn::Error::new(
                $span,
                $msg
            ).to_compile_error()
        )
    };
}

#[proc_macro_derive(NumberEnum)]
pub fn derive_number_enum(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let temp_span = input.ident.span().to_owned();

    let enum_name = input.ident.to_owned();
    let repr = if let Some(_repr) = get_repr_size(input.attrs.iter()) {
        _repr
    } else {
        rt_error!(temp_span, "Expect an exact and right `repr` arugment")
    };

    let data = match input.data {
        syn::Data::Enum(data_enum) => data_enum,
        syn::Data::Struct(_) | syn::Data::Union(_) => {
            rt_error!(input.ident.span(), "Only Enum can derive `NumberEnum`")
        },
    };

    let impl_convert = impl_convert_traits(&enum_name, &data, &repr);
    let impl_calc = impl_calc_traits(&enum_name, &repr);

    proc_macro::TokenStream::from(quote! {
        #impl_convert

        #impl_calc
    })
}

fn impl_convert_traits(name: &Ident, data: &DataEnum, repr: &Ident) -> TokenStream {
    let mut enum_to_number: Vec<TokenStream> = Vec::new();
    let mut number_to_enum: Vec<TokenStream> = Vec::new();
    let mut current_number = 0;

    for item in data.variants.iter() {
        let variant_name = &item.ident;
        enum_to_number.push(quote!(
            Self::#variant_name => #current_number as #repr
        ));

        if repr == "u8" {
            let convert_num = current_number as u8;
            number_to_enum.push(quote!(
                #convert_num => Self::#variant_name
            ));            
        } else {
            let convert_num = current_number as u16;
            number_to_enum.push(quote!(
                #convert_num => Self::#variant_name
            ));
        }

        current_number += 1;
    }

    quote! {
        impl Into<#repr> for #name {
            fn into(self) -> #repr {
                match self {
                    #(#enum_to_number),*
                }
            }
        }

        impl From<#repr> for #name {
            fn from(value: #repr) -> Self {
                match value {
                    #(#number_to_enum),*,
                    _ => {
                        panic!("Failed to parse number into enum!")
                    }
                }
            }
        }
    }
}

fn impl_calc_traits(name: &Ident, repr: &Ident) -> TokenStream {
    quote! {
        impl std::ops::Add for #name {
            type Output = Self;

            fn add(self, rhs: Self) -> Self::Output {
                let enum_num: #repr = self.into();
                let rhs_num: #repr = rhs.into();
                (enum_num + rhs_num).into()
            }
        }

        impl std::ops::Sub for #name {
            type Output = Self;

            fn sub(self, rhs: Self) -> Self::Output {
                let enum_num: #repr = self.into();
                let rhs_num: #repr = rhs.into();
                (enum_num - rhs_num).into()
            }
        }

        impl std::ops::AddAssign for #name {
            fn add_assign(&mut self, rhs: Self) {
                *self = *self + rhs;
            }
        }

        impl std::ops::SubAssign for #name {
            fn sub_assign(&mut self, rhs: Self) {
                *self = *self - rhs;
            }
        }
    }
}

/// Get the referred size for enum number.
fn get_repr_size<'a, A>(attrs: A) -> Option<Ident>
where A: Iterator<Item = &'a Attribute>
{
    for attr in attrs {
        if let Meta::List(ref meta_list) = attr.meta {
            if let Some(ident) = meta_list.path.get_ident() {
                if ident == "repr" {
                    let mut nested = meta_list.tokens.to_owned().into_iter();
                    let repr_tree = match (nested.next(), nested.next()) {
                        (Some(repr_tree), None) => repr_tree,
                        _ => return None
                    };

                    let repr_ident: Ident = parse_quote! {
                        #repr_tree
                    };

                    if repr_ident == "C" {
                        return None
                    }

                    return Some(repr_ident)
                }
            }
        }
    }

    None
}
