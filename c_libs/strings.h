//! Create and manage Rust-style strings as fat pointers + a heap allocated buffer.

#include <stdlib.h>
#include <string.h>
#include <stdbool.h>

//! String struct that holds both the data pointer and metadata
typedef struct {
    char* data;      // Heap-allocated buffer
    size_t len;      // Length of the string in bytes
    size_t capacity; // Total allocated capacity
} String;

// Initialize a new string from a C string
String string_from(const char* cstr) {
    size_t len = strlen(cstr);
    size_t capacity = len + 1;  // +1 for internal null terminator
    
    char* buf = malloc(capacity);
    memcpy(buf, cstr, len + 1);  // Copy including null terminator
    
    return (String){
        .data = buf,
        .len = len,
        .capacity = capacity
    };
}

// Create an empty string with given capacity
String string_with_capacity(size_t capacity) {
    char* buf = malloc(capacity);
    buf[0] = '\0';  // Null terminate empty string
    
    return (String){
        .data = buf,
        .len = 0,
        .capacity = capacity
    };
}

// Free the string's memory
void string_free(String* str) {
    free(str->data);
    str->data = NULL;
    str->len = 0;
    str->capacity = 0;
}

// Ensure the string has enough capacity for additional bytes
static void ensure_capacity(String* str, size_t additional) {
    size_t required = str->len + additional + 1;  // +1 for null terminator
    if (required <= str->capacity) return;
    
    // Grow by doubling or required amount, whichever is larger
    size_t new_capacity = str->capacity * 2;
    if (new_capacity < required) new_capacity = required;
    
    char* new_buf = realloc(str->data, new_capacity);
    str->data = new_buf;
    str->capacity = new_capacity;
}

// Append another string
void string_append(String* str, const String* other) {
    ensure_capacity(str, other->len);
    memcpy(str->data + str->len, other->data, other->len + 1);
    str->len += other->len;
}

// Get a slice of the string (returns new string)
String string_slice(const String* str, size_t start, size_t end) {
    if (end > str->len) end = str->len;
    if (start > end) start = end;
    
    size_t slice_len = end - start;
    String result = string_with_capacity(slice_len + 1);
    
    memcpy(result.data, str->data + start, slice_len);
    result.data[slice_len] = '\0';
    result.len = slice_len;
    
    return result;
}

// Compare two strings
int string_compare(const String* a, const String* b) {
    size_t min_len = a->len < b->len ? a->len : b->len;
    int cmp = memcmp(a->data, b->data, min_len);
    
    if (cmp == 0) {
        if (a->len < b->len) return -1;
        if (a->len > b->len) return 1;
        return 0;
    }
    
    return cmp;
}

// Get character at index (assumes valid index)
char string_char_at(const String* str, size_t index) {
    return str->data[index];
}

// Get C string pointer (for interop)
const char* string_as_cstr(const String* str) {
    return str->data;
}