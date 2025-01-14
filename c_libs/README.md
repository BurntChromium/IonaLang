# C Libraries for Iona

The Iona runtime is written in C, and the Iona compiler emits C code. Some of that C code needs to be use concepts which aren't present in the Iona language (such as atomics and raw pointers), so we can't rely on the compiler to emit it. These libraries implement the necessary low level functionality which is then shadowed by the Iona standard libraries. 