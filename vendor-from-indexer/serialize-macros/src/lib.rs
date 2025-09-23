//! Derive macros for `midnight-serialize`.
extern crate proc_macro;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{
    Data, DeriveInput, Fields, GenericParam, Generics, Index, parse_macro_input, parse_quote,
};

#[proc_macro_derive(Versioned)]
pub fn derive_versioned(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics Versioned for #name #ty_generics #where_clause {
            const VERSION: Option<serialize::Version> = None;
        }
    };

    proc_macro::TokenStream::from(expanded)
}

// Macro to implement Deserializable for an unversioned object made up entirely of serializable
// objects
#[proc_macro_derive(Deserializable)]
pub fn derive_deserializable(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let generics = deserializable_add_trait_bounds(input.generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let deserialize = deserialize(&input.data);

    let expanded = quote! {
        impl #impl_generics Deserializable for #name #ty_generics #where_clause {
            fn versioned_deserialize<R: std::io::Read>(reader: &mut R, version: Option<&serialize::Version>, recursion_depth: u32) -> Result<Self, std::io::Error> {
                #deserialize
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}

fn deserializable_add_trait_bounds(mut generics: Generics) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(parse_quote!(Deserializable));
        }
    }
    generics
}

// Macro to implement Serializable for an unversioned object made up entirely of serializable
// objects
#[proc_macro_derive(Serializable)]
pub fn derive_serializable(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let generics = serializable_add_trait_bounds(input.generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let serialize = serialize(&input.data);
    let size = size(&input.data);

    let expanded = quote! {
        impl #impl_generics Serializable for #name #ty_generics #where_clause {
            fn unversioned_serialize<W: std::io::Write>(value: &Self, writer: &mut W) -> Result<(), std::io::Error> {
                #serialize
                Ok(())
            }

            fn unversioned_serialized_size(value: &Self) -> usize {
                #size
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}

fn serializable_add_trait_bounds(mut generics: Generics) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(parse_quote!(Serializable));
        }
    }
    generics
}

fn serialize_fields(fields: &Fields) -> TokenStream {
    match fields {
        Fields::Named(ref fields) => {
            // Expands to an expression like
            //      <A as Serializable>::serialize(a, writer)?;
            //      <B as Serializable>::serialize(b, writer)?;
            let recurse = fields.named.iter().map(|f| {
                let name = &f.ident;
                let ty = &f.ty;
                quote_spanned! {f.span()=>
                    <#ty as Serializable>::serialize(#name, writer)?;
                }
            });
            quote! {
                #(#recurse)*
            }
        }
        Fields::Unnamed(ref fields) => {
            let recurse = fields.unnamed.iter().enumerate().map(|(i, f)| {
                let name = Ident::new(&format!("var_{}", i), Span::call_site());
                let ty = &f.ty;
                quote_spanned! {f.span()=>
                    <#ty as Serializable>::serialize(#name, writer)?;
                }
            });
            quote! {
                #(#recurse)*
            }
        }
        Fields::Unit => TokenStream::new(),
    }
}

fn unpack_struct(fields: &Fields) -> TokenStream {
    // Expands to
    // let a = &value.a;
    // let b = &value.b;
    match fields {
        Fields::Named(ref fields) => {
            let recurse = fields.named.iter().map(|var| {
                let name = &var.ident;
                quote_spanned!(var.span()=>
                    let #name = &value.#name;
                )
            });
            quote! {
                #(#recurse)*
            }
        }
        Fields::Unnamed(ref fields) => {
            let recurse = fields.unnamed.iter().enumerate().map(|(i, var)| {
                let name = Ident::new(&format!("var_{}", i), Span::call_site());
                let index = Index::from(i);
                quote_spanned!(var.span()=>
                    let #name = &value.#index;
                )
            });
            quote! {
                #(#recurse)*
            }
        }
        Fields::Unit => TokenStream::new(),
    }
}

fn unpack_enum(fields: &Fields) -> TokenStream {
    // Expands to
    // (a, b)
    match fields {
        Fields::Named(ref fields) => {
            let recurse = fields.named.iter().map(|var| {
                let name = &var.ident;
                quote_spanned!(var.span()=>
                    #name,
                )
            });
            quote! {
                {#(#recurse)*}
            }
        }
        Fields::Unnamed(ref fields) => {
            let recurse = fields.unnamed.iter().enumerate().map(|(i, var)| {
                let name = Ident::new(&format!("var_{}", i), Span::call_site());
                quote_spanned!(var.span()=>
                    #name,
                )
            });
            quote! {
                (#(#recurse)*)
            }
        }
        Fields::Unit => TokenStream::new(),
    }
}

fn serialize(data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => {
            let unpack = unpack_struct(&data.fields);
            let fields = serialize_fields(&data.fields);
            quote! {
                #unpack #fields
            }
        }
        Data::Enum(ref data) => {
            let recurse = data.variants.iter().enumerate().map(|(i, var)| {
                let fields = serialize_fields(&var.fields);
                let unpack = unpack_enum(&var.fields);
                let ty = &var.ident;
                quote_spanned! {var.span()=>
                    Self::#ty #unpack => {
                        <u32 as Serializable>::serialize(&(#i as u32), writer)?;
                        #fields
                    },
                }
            });
            quote! {
                match value {
                    #(#recurse)*
                }
            }
        }
        Data::Union(_) => TokenStream::new(),
    }
}

fn size_fields(fields: &Fields) -> TokenStream {
    match fields {
        Fields::Named(ref fields) => {
            // Expands to an expression like
            //      0 + <A as Serializable>::serialized_size(a)
            //      + <B as Serializable>::serialized_size(b)
            let recurse = fields.named.iter().map(|f| {
                let name = &f.ident;
                let ty = &f.ty;
                quote_spanned! {f.span()=>
                    + <#ty as Serializable>::serialized_size(#name)
                }
            });
            quote! {
                0 #(#recurse)*
            }
        }
        Fields::Unnamed(ref fields) => {
            let recurse = fields.unnamed.iter().enumerate().map(|(i, f)| {
                let name = Ident::new(&format!("var_{}", i), Span::call_site());
                let ty = &f.ty;
                quote_spanned! {f.span()=>
                    + <#ty as Serializable>::serialized_size(#name)
                }
            });
            quote! {
                0 #(#recurse)*
            }
        }
        Fields::Unit => quote! { 0 },
    }
}

fn size(data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => {
            let unpack = unpack_struct(&data.fields);
            let fields = size_fields(&data.fields);
            quote! {
                #unpack #fields
            }
        }
        Data::Enum(ref data) => {
            let recurse = data.variants.iter().map(|var| {
                let unpack = unpack_enum(&var.fields);
                let fields = size_fields(&var.fields);
                let ty = &var.ident;
                quote_spanned! {var.span()=>
                    Self::#ty #unpack => {
                        4 + #fields
                    }
                }
            });
            quote! {
                match value {
                    #(#recurse)*
                }
            }
        }
        Data::Union(_) => unimplemented!(),
    }
}

fn deserialize_fields(fields: &Fields) -> TokenStream {
    match fields {
        Fields::Named(ref fields) => {
            // Expands to an expression like
            //      a: <A as Deserializable>::deserialize(reader)?,
            //      b: <B as Deserializable>::deserialize(reader)?,
            let recurse = fields.named.iter().map(|f| {
                let name = &f.ident;
                let ty = &f.ty;
                quote_spanned! {f.span()=>
                    #name: <#ty as Deserializable>::deserialize(reader, recursion_depth)?,
                }
            });
            quote! {
                {#(#recurse)*}
            }
        }
        Fields::Unnamed(ref fields) => {
            let recurse = fields.unnamed.iter().map(|f| {
                let ty = &f.ty;
                quote_spanned! {f.span()=>
                    <#ty as Deserializable>::deserialize(reader, recursion_depth)?,
                }
            });
            quote! {
                (#(#recurse)*)
            }
        }
        Fields::Unit => quote! {},
    }
}

fn deserialize(data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => {
            let fields = deserialize_fields(&data.fields);
            quote! {
                Ok(Self #fields)
            }
        }
        Data::Enum(ref data) => {
            let recurse = data.variants.iter().enumerate().map(|(i, var)| {
                let i = i as u32;
                let fields = deserialize_fields(&var.fields);
                let name = &var.ident;
                quote_spanned! {var.span()=>
                    #i => Ok(Self::#name #fields),
                }
            });
            quote! {
                let discriminant = <u32 as Deserializable>::deserialize(reader, recursion_depth)?;
                match discriminant {
                    #(#recurse)*
                    _ => Err(Self::deserialization_error(None, "Unrecognised discriminant".to_string()))
                }
            }
        }
        Data::Union(_) => unimplemented!(),
    }
}
