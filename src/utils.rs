/*
 *  Copyright 2021 QuantumBadger
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

use std::time::Instant;

pub(crate) struct BenchmarkTimer {
    name: String,
    start_time: Instant,
}

impl BenchmarkTimer {
    #[allow(dead_code)]
    pub(crate) fn start<S: AsRef<str>>(name: S) -> Self {
        log::info!("[Benchmark] [START] {}", name.as_ref());

        BenchmarkTimer {
            name: name.as_ref().to_string(),
            start_time: Instant::now(),
        }
    }
}

impl Drop for BenchmarkTimer {
    fn drop(&mut self) {
        log::info!(
            "[Benchmark] [END]   {} (took {:.3} ms)",
            self.name,
            self.start_time.elapsed().as_secs_f64() * 1000.0
        );
    }
}
