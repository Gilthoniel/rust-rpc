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
    Request::#name(arg) => Response::#name(self.#func_name(arg))
  }
}

/// Produce the client functions that will make the requests to
/// the servers.
fn derive_client_func(sig: &Signature, name: &Ident, param: &Type, out: &Type) -> ItemFn {
  let func_name = &sig.ident;

  syn::parse_quote! {
    pub fn #func_name(&self, arg: #param) -> Result<#out, Error> {
      let r = self.send_message(Request::#name(arg));
        
      match r {
        Response::#name(value) => Ok(value),
        _ => Err(Error::NoMatchingResponse),
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

        let out: &Type;
        if let syn::ReturnType::Type(_, ref t) = &m.sig.output {
          out = t;
        } else {
          panic!("rpc function expects one return type");
        }

        requests.push(derive_variante(name, param));
        responses.push(derive_variante(name, out));
        methods.push(TraitItem::Method(m.clone()));
        handlers.push(derive_handler_arm(&m.sig, name));
        client_funcs.push(derive_client_func(&m.sig, name, param, out));
      },
      _ => (), // only interested in methods
    }
  }

  let result = quote! {
    pub type RequestProcessor<Req, Rep> = dyn FnMut(Req) -> Rep + Send + Sync;

    #[derive(Debug)]
    pub enum Error {
      NoMatchingResponse
    }

    /// Request enumerates the list of possible request messages sent
    /// by clients to a server to execute a request.
    #[derive(Serialize, Deserialize)]
    pub enum Request { #requests }

    /// Response enumerates the list of possible response messages sent
    /// by the server to a client after processing a request.
    #[derive(Serialize, Deserialize)]
    pub enum Response { #responses }

    pub trait #name_service: Sized + Sync + Send + 'static {
      #(#methods)*

      fn get_processor(mut self) -> Box<RequestProcessor<Request, Response>> {
        Box::new(move |msg| match msg {
          #(#handlers)*
        })
      }
    }

    pub struct #client_name {
      s: Connection<Request, Response>,
    }

    impl #client_name {
      fn new(t: impl ClientTransport<Request, Response>) -> #client_name {
        #client_name { s: t.connect() }
      }

      fn send_message(&self, msg: Request) -> Response {
        self.s.send(msg)
      }

      #(#client_funcs)*
    }
  };

  result.into()
}
