# Pyrocord Route Derive

Example:

```rs
use reqwest::Method;
use pyrocord_route_derive::Routes;

#[derive(Routes)]
pub enum Route {
    #[route(GET, "/applications/{application_id}/commands")]
    GetGlobalApplicationCommands(u64),
}
```
