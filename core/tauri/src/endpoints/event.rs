// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use crate::{endpoints::InvokeResponse, runtime::Runtime, sealed::ManagerBase, Manager, Window};
use serde::Deserialize;

/// The API descriptor.
#[derive(Deserialize)]
#[serde(tag = "cmd", rename_all = "camelCase")]
pub enum Cmd {
  /// Listen to an event.
  Listen { event: String, handler: String },
  /// Unlisten to an event.
  #[serde(rename_all = "camelCase")]
  Unlisten { event_id: u64 },
  /// Emit an event to the webview associated with the given window.
  /// If the window_label is omitted, the event will be triggered on all listeners.
  #[serde(rename_all = "camelCase")]
  Emit {
    event: String,
    window_label: Option<String>,
    payload: Option<String>,
  },
}

impl Cmd {
  pub fn run<R: Runtime>(self, window: Window<R>) -> crate::Result<InvokeResponse> {
    match self {
      Self::Listen { event, handler } => {
        let event_id = rand::random();
        window.eval(&listen_js(&window, event.clone(), event_id, handler), |_| ())?;
        window.register_js_listener(event, event_id);
        Ok(event_id.into())
      }
      Self::Unlisten { event_id } => {
        window.eval(&unlisten_js(&window, event_id), |_| ())?;
        window.unregister_js_listener(event_id);
        Ok(().into())
      }
      Self::Emit {
        event,
        window_label,
        payload,
      } => {
        // dispatch the event to Rust listeners
        window.trigger(&event, payload.clone());

        if let Some(target) = window_label {
          window.emit_to(&target, &event, payload)?;
        } else {
          window.emit_all(&event, payload)?;
        }
        Ok(().into())
      }
    }
  }
}

pub fn unlisten_js<R: Runtime>(window: &Window<R>, event_id: u64) -> String {
  format!(
    "
      for (var event in (window['{listeners}'] || {{}})) {{
        var listeners = (window['{listeners}'] || {{}})[event]
        if (listeners) {{
          window['{listeners}'][event] = window['{listeners}'][event].filter(function (e) {{ return e.id !== {event_id} }})
        }}
      }}
    ",
    listeners = window.manager().event_listeners_object_name(),
    event_id = event_id,
  )
}

pub fn listen_js<R: Runtime>(
  window: &Window<R>,
  event: String,
  event_id: u64,
  handler: String,
) -> String {
  format!(
    "if (window['{listeners}'] === void 0) {{
      window['{listeners}'] = Object.create(null)
    }}
    if (window['{listeners}']['{event}'] === void 0) {{
      window['{listeners}']['{event}'] = []
    }}
    window['{listeners}']['{event}'].push({{
      id: {event_id},
      handler: window['{handler}']
    }});
  ",
    listeners = window.manager().event_listeners_object_name(),
    event = event,
    event_id = event_id,
    handler = handler
  )
}
