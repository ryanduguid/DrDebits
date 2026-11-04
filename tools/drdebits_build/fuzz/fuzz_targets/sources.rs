// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: LicenseRef-Duguid-Public-Attribution-1.0

//! Fabricated bytes to the YAML source reader and the decision-status reader.
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    if let Err(problem) = drdebits::fuzz::check_input(data) {
        panic!("{problem}");
    }
});
