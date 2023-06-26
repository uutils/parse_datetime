// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let s = std::str::from_utf8(data).unwrap_or("");
    let _ = parse_datetime::from_str(s);
});
