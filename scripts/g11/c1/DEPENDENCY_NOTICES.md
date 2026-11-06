# Dependency build adaptation

`base-arm64-clang17.patch` changes only the CPU flag detection in Jane Street
`base` v0.16.3 (https://github.com/janestreet/base/tree/v0.16.3). It restricts
the x86 `-mpopcnt` probe to x86 OCaml targets. It does not change Belenios,
ElGamal, proofs, hashing, encodings, thresholds or election semantics.

Upstream base archive SHA-256:
`9bf9e503e2bd010325c7b0f5ae007ce1e5d3da61f0e06d2fbb64e9d12ccff93c`.

The upstream base license is reproduced below for the patched excerpt.

The MIT License

Copyright (c) 2016--2023 Jane Street Group, LLC <opensource-contacts@janestreet.com>

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
