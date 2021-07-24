//! login_required macros for actix-loginmanager
//! # Example
//! ```rust
//! use actix_loginmanager::login_required;
//! // define or import `user` which implements `UserMinix` trait.
//! 
//! #[login_required(User)]
//! async fn hello()->impl actix_web::Responder{
//!     user.is_actived(); //can access user:Rc<User>
//!     return "hello";
//! }
//! 
//! #[login_required(User, name="user")]
//! async fn hello()->impl actix_web::Responder{
//!     user.is_actived(); //can access user:Rc<User>
//!     return "hello";
//! }
//! ```

use proc_macro::TokenStream;

/// inject an argument `UserWrapAuth(UserWrap(user)): UserWrapAuth<User>` into the function.
/// 
/// # Syntax
/// ```text
/// #[login_required(UserType,name="user")]
/// ```
/// 
/// # Attributes
/// - `UserType` - Define the variable type.
/// - `name="user"` - Define the variable name.
/// 
/// # Example
/// ```rust
/// #[login_required(User)]
/// async fn hello()->impl actix_web::Responder{
///     user.is_actived(); //can access user:Rc<User>
///     return "hello";
/// }
/// ```
#[proc_macro_attribute]
pub fn login_required(args: TokenStream, item: TokenStream) -> TokenStream {
    use quote::quote;
    use syn::{NestedMeta,Lit,Meta};
    let args = syn::parse_macro_input!(args as syn::AttributeArgs);
    let mut user = None;
    let mut name = "user".to_owned();
    for arg in args{
        match arg{
            NestedMeta::Lit(Lit::Str(lit))=> match user{
                None=>{user = Some(lit.value());},
                _=>{
                    return syn::Error::new_spanned(lit,"The user type cannot be defined twice")
                        .to_compile_error()
                        .into();
                }
            },
            NestedMeta::Meta(Meta::Path(path))=>match user{
                None=>{
                    user = Some(path.segments.first().unwrap().ident.clone().to_string());
                },
                _=>{
                    return syn::Error::new_spanned(path,"The user type cannot be defined twice")
                        .to_compile_error()
                        .into();
                }
            },
            NestedMeta::Meta(Meta::NameValue(nv))=>{
                if &nv.path.segments.first().unwrap().ident.clone().to_string() == "name"{
                    if let Lit::Str(lit) = nv.lit{
                        name = lit.value();
                    }
                }
            },
            _=>{

            }
        }
    }
    if user == None{
        return syn::Error::new_spanned("#[login_required]", "need user type,Ex:#[login_required(User)]")
            .to_compile_error()
            .into();
    }

    let mut input = syn::parse_macro_input!(item as syn::ItemFn);
    let attrs = &input.attrs;
    let vis = &input.vis;
    let sig = &mut input.sig;
    let body = &input.block;
    let param = format!("actix_loginmanager::UserWrapAuth(actix_loginmanager::UserWrap({})): actix_loginmanager::UserWrapAuth<{}>",name,user.unwrap());
    let param =  syn::parse_str(&param).unwrap();
    sig.inputs.push(param);

    (quote! {
        #(#attrs)*
        #vis #sig {
            #body
        }
    })
    .into()
}