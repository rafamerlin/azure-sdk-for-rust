// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

#![doc = include_str!("../README.md")]

pub mod models;

#[doc(hidden)]
pub mod rfc3339;

#[cfg(feature = "shared_key")]
pub mod shared_key;
