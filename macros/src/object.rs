use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

pub fn expand(input: DeriveInput) -> TokenStream {
    let DeriveInput {
        ident, generics, ..
    } = input;
    let (impl_generics, type_generics, where_clause) =
        generics.split_for_impl();
    let output = quote! {
        #[automatically_derived]
        impl#impl_generics ::entrust::Object for #ident#type_generics #where_clause {
            fn to_document(&self) -> ::anyhow::Result<::bson::Document> {
                let doc = ::bson::to_document(self)?;
                let doc = ::entrust::util::camelize_document(doc);
                Ok(doc)
            }

            fn from_document(mut doc: ::bson::Document) -> ::anyhow::Result<Self> {
                let doc = ::entrust::util::snakeify_document(doc);
                let object = ::bson::from_document(doc)?;
                Ok(object)
            }
        }
    };
    output.into()
}
