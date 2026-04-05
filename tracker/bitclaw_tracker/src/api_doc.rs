use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(info(title = "bitclaw-tracker API",), paths(), components(schemas(),))]
pub struct ApiDoc;
