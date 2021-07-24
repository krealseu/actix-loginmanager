login_required macros for actix-loginmanager

# Syntax
```text
#[login_required(UserType,name="user")]
```

# Attributes
- `UserType` - Define the variable type.
- `name="user"` - Define the variable name.

# Example
```rust
use actix_loginmanager::login_required;
// define or import `user` which implements `UserMinix` trait.

#[login_required(User)]
async fn hello()->impl actix_web::Responder{
    user.is_actived(); //can access user:Rc<User>
    return "hello";
}

#[login_required(User,name="user")]
async fn hello()->impl actix_web::Responder{
    user.is_actived(); //can access user:Rc<User>
    return "hello";
}
```