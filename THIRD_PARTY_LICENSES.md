# Third-Party Licenses & Acknowledgements

The `wstrace` software is licensed under the **GNU General Public License v3.0 (GPL-3.0-or-later)**.
In accordance with open-source licensing standards and best practices, this document acknowledges the third-party open-source libraries incorporated into the binary distribution of `wstrace`.

All third-party libraries utilized by `wstrace` are distributed under permissive licenses (**MIT** or **Apache-2.0** dual license), which are fully compatible with the GNU General Public License v3.0.

---

## Summary of Direct Dependencies

| Crate | Purpose | License | Copyright / Authors |
| :--- | :--- | :--- | :--- |
| **`ratatui`** | Terminal User Interface (TUI) layout & rendering | MIT | Ratatui Developers |
| **`crossterm`** | Cross-platform terminal manipulation & input handling | MIT | Timon Post & Contributors |
| **`windows-sys`** | Windows API & Kernel ETW raw bindings | MIT OR Apache-2.0 | Microsoft Corporation |
| **`clap`** | Command-line argument parsing | MIT OR Apache-2.0 | Clap Developers |
| **`crossbeam-channel`** | High-performance lock-free event queue channels | MIT OR Apache-2.0 | Crossbeam Developers |
| **`serde`** | Serialization & deserialization framework | MIT OR Apache-2.0 | Erick Tryzelaar & David Tolnay |
| **`serde_json`** | JSON export formatter | MIT OR Apache-2.0 | Erick Tryzelaar & David Tolnay |
| **`chrono`** | High-precision microsecond timestamp management | MIT OR Apache-2.0 | Chrono Developers |
| **`anyhow`** | Idiomatic error handling & diagnostic context | MIT OR Apache-2.0 | David Tolnay |

---

## License Texts

### The MIT License (MIT)

```text
Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

---

### The Apache License, Version 2.0

```text
Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```
