//! This crate emulates warnings similar to [Procedural Macro Diagnostics](https://github.com/rust-lang/rust/issues/54140) on stable Rust.
//!
//! # Limitations
//!
//! - Only warnings can be generated by this crate, and these warnings are formally `deprecated`.
//! - Only one simple [`Span`] is accepted per warning.
//! - Compound warnings are not supported.
//! - The procedural macro needs to find an appropriate spot where to potentially emit a generated module.
//!
//! # Upgrading to Procedural Macro Diagnostics
//!
//! Once this crate is somewhat stable, it will be able to automatically upgrade to the proper warning mechanism. At this point, any versions unable to do so will be yanked whenever a newer more stable version exists.
//!
//! Once procedural macro diagnostics are stabilised, this crate will be updated with versions marked deprecated themselves.

#![doc(html_root_url = "https://docs.rs/proc-macro-warning/0.0.1")]
#![warn(clippy::pedantic)]
// Switching this on automatically isn't possible until <https://github.com/rust-lang/rust/issues/54726> lands.
#![cfg_attr(all(feature = "nightly", proc_macro), feature(proc_macro_diagnostic))]

#[cfg(doctest)]
pub mod readme {
	doc_comment::doctest!("../README.md");
}

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote_spanned;
use std::{mem, sync::Mutex};

thread_local! {
	static WARNINGS: Mutex<Vec<(Span, TokenStream)>> = Mutex::default();
}

#[cfg(all(feature = "nightly", proc_macro))]
extern crate proc_macro;

#[cfg(all(feature = "nightly", proc_macro))]
pub fn warn(span: Span, message: &str) {
	proc_macro::Diagnostic::spanned(span.unwrap(), proc_macro::Level::Warning, message).emit()
}

#[cfg(any(not(feature = "nightly"), not(proc_macro)))]
pub fn warn(span: Span, message: &str) {
	use proc_macro2::Literal;

	let mut message = Literal::string(message);
	message.set_span(span);
	WARNINGS.with(move |warnings| {
		warnings.lock().unwrap().push((
			span,
			quote_spanned! {span=>
				#[deprecated = #message]
				struct Warning;
				struct Warner(Warning);
			},
		))
	})
}

#[must_use]
pub fn collect_warning_statements() -> TokenStream {
	WARNINGS
		.with(|warnings| mem::replace(&mut *warnings.lock().unwrap(), vec![]))
		.into_iter()
		.map(|(span, warning_items)| {
			quote_spanned! {span=>
				let _ = { #warning_items };
			}
		})
		.collect()
}

#[must_use]
pub fn collect_warning_items() -> TokenStream {
	WARNINGS
		.with(|warnings| mem::replace(&mut *warnings.lock().unwrap(), vec![]))
		.into_iter()
		.map({
			let mut count = 0;
			move |(span, warning_items)| {
				// Span::mixed_site() *may* be nicer, but it's not available in Rust 1.40.0.
				let mod_ident = Ident::new(&format! {"W{}", count}, span);
				count += 1;
				quote_spanned! {span=>
					mod #mod_ident { #warning_items }
				}
			}
		})
		.collect()
}
