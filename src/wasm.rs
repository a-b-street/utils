use wasm_bindgen::{JsCast, JsValue};

/// Download a file to the browser, useful for debugging.
pub fn download_string(contents: &str, filename: &str) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;

    let url = format!(
        "data:text/plain;charset=utf-8,{}",
        urlencoding::encode(contents)
    );
    let a = document
        .create_element("a")?
        .dyn_into::<web_sys::HtmlAnchorElement>()?;
    a.set_href(&url);
    a.set_download(filename);

    let body = document
        .body()
        .ok_or_else(|| JsValue::from_str("no body"))?;
    body.append_child(&a)?;
    a.click();
    let _ = body.remove_child(&a)?;

    Ok(())
}
