extern crate proc_macro;
extern crate syn;
extern crate quote;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
  TraitItem,
  Variant,
  Signature,
  FnArg,
  Arm,
  Type,
  Ident,
  ItemFn,
  PathArguments,
  GenericArgument,
  punctuated::Punctuated,
  token::Comma,
};

/// Produce an enumeration using the name and the type provided.
fn derive_variante(name: &Ident, param: &Type) -> Variant {
  syn::parse_quote! { #name(#param) }
}

/// Produce the match pattern of the rpc requests. Each request
/// is handled by the rpc implementation and wrapped around
/// a response message.
fn derive_handler_arm(sig: &Signature, name: &Ident) -> Arm {
  let func_name = &sig.ident;

  syn::parse_quote! {
    ClientData::#name(arg) => {
      let result = self.#func_name(arg);

      match result {
        Ok(value) => ServerData::#name(value),
        Err(e) => ServerData::Error(e),
      }
    }
  }
}

/// Produce the client functions that will make the requests to
/// the servers.
fn derive_client_func(sig: &Signature, name: &Ident, param: &Type, out: &Type, err_type: &Type) -> ItemFn {
  let func_name = &sig.ident;

  syn::parse_quote! {
    pub fn #func_name(&self, arg: #param) -> Result<#out, #err_type> {
      let data = self.send_message(ClientData::#name(arg))?;

      match data {
        ServerData::#name(value) => Ok(value),
        ServerData::Error(e) => Err(e),
        _ => panic!("invalid response type"),
      }
    }
  }
}

#[proc_macro_attribute]
pub fn service(_: TokenStream, item: TokenStream) -> TokenStream {
  let input = syn::parse_macro_input!(item as syn::ItemTrait);

  let name_service = &input.ident;
  let client_name = Ident::new(format!("{}Client", name_service).as_ref(), syn::export::Span::call_site());
  let mut requests: Punctuated<Variant, Comma> = Punctuated::new();
  let mut responses: Punctuated<Variant, Comma> = Punctuated::new();
  let mut methods: Vec<TraitItem> = Vec::new();
  let mut handlers: Vec<Arm> = Vec::new();
  let mut client_funcs: Vec<ItemFn> = Vec::new();
  let mut err_type = None;

  for method in input.items {
    match method {
      TraitItem::Method(m) => {
        let name = m.sig.ident.to_string();
        let name = name[0..1].to_uppercase() + &name[1..];
        let name = &Ident::new(name.as_ref(), syn::export::Span::call_site());

        let param: &Type;
        if let FnArg::Typed(ref pt) = &m.sig.inputs[1] {
            param = &pt.ty;
        } else {
          panic!("rpc function expects one argument");
        }

        let mut out = None;
        if let syn::ReturnType::Type(_, t) = &m.sig.output {
          if let Type::Path(t) = t.as_ref() {
            let t = &t.path.segments.last().unwrap().arguments;

            if let PathArguments::AngleBracketed(t) = t {
              let mut iter = t.args.iter();

              if let Some(GenericArgument::Type(t)) = iter.next() {
                out = Some(t);
              }

              if let Some(GenericArgument::Type(t)) = iter.next() {
                err_type = Some(t.clone());
              }
            }
          }
        }

        let out = out.expect("rpc function expects a Result<T, E> type or nothing as return");
        let err_type = err_type.clone().unwrap();

        requests.push(derive_variante(name, param));
        responses.push(derive_variante(name, out));
        methods.push(TraitItem::Method(m.clone()));
        handlers.push(derive_handler_arm(&m.sig, name));
        client_funcs.push(derive_client_func(&m.sig, name, param, out, &err_type));
      },
      _ => (), // only interested in methods
    }
  }

  let err_type = err_type.expect("rpc function expects an error.");

  let result = quote! {
    use serde::{Deserialize, Serialize};
    use rpc::transport::ClientTransport;
    use std::error::Error;
    use std::panic::RefUnwindSafe;

    /// ClientData enumerates the list of possible request messages sent
    /// by clients to a server to execute a request.
    #[derive(Serialize, Deserialize, Debug)]
    pub enum ClientData { #requests }

    /// ServerData enumerates the list of possible response messages sent
    /// by the server to a client after processing a request.
    #[derive(Serialize, Deserialize, Debug)]
    pub enum ServerData {
      Error(#err_type),
      #responses
    }

    pub type RequestProcessor = dyn Fn(ClientData) -> ServerData + Send + Sync + RefUnwindSafe;

    pub trait #name_service: Sized + Sync + Send + RefUnwindSafe + 'static {
      #(#methods)*

      fn get_processor(self) -> Box<RequestProcessor> {
        Box::new(move |msg| match msg {
          #(#handlers),*
        })
      }
    }

    pub struct #client_name<T> {
      t: T,
    }

    impl<T> #client_name<T>
    where
      T: ClientTransport<ClientData, ServerData>,
    {
      pub fn new(t: T) -> #client_name<T> {
        #client_name { t }
      }

      fn send_message(&self, msg: ClientData) -> Result<ServerData, #err_type> {
        let res = self.t.send(msg)?;
        
        Ok(res)
      }

      #(#client_funcs)*
    }
  };

  result.into()
}
