use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use er::*;

#[derive(Er)]
pub struct NotFoundErr(pub String);
pub fn lookup(name: &str) -> ErResult<&str, NotFoundErr> {
    if name == "HaandboldFuglen" {
        Ok(name)
    } else {
        er_bail!(NotFoundErr::new(name));
    }
}

#[derive(Er)]
#[er(wrap)]
pub struct HandlerErr;
pub fn handler(name: &str) -> Result<&str, HandlerErrWrap> {
    let value = lookup(name).er(())?;
    Ok(value)
}

// Wrap gives us a local type for Axum's trait.
impl IntoResponse for HandlerErrWrap {
    fn into_response(self) -> Response {
        let status = if self.er_contains::<NotFoundErr>() {
            StatusCode::NOT_FOUND
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        };

        (status, self.er_top().to_string()).into_response()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    pub fn boundary() {
        let response = handler("HaandboldFuglen").into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let response = handler("missing").into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let error = HandlerErr::new().er_wrap();
        assert_eq!(
            error.into_response().status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
