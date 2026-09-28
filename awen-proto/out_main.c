#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <stdlib.h>
static long al_alloc_count = 0;
static void* (*al_real_malloc)(size_t) = malloc;
static void* al_counted_malloc(size_t n) { al_alloc_count++; return al_real_malloc(n); }
#define malloc al_counted_malloc
static void* (*al_real_realloc)(void*, size_t) = realloc;
static void* al_counted_realloc(void* p, size_t n) { al_alloc_count++; return al_real_realloc(p, n); }
#define realloc al_counted_realloc
static void al_alloc_report(void) { const char* e = getenv("AINE_ALLOC_STATS"); if (e) { fprintf(stderr, "[aine] heap allocations: %ld\n", al_alloc_count); } }
__attribute__((constructor)) static void al_alloc_setup(void) { atexit(al_alloc_report); }
typedef struct { size_t len; size_t elem_size; void* data; } al_vec;
al_vec al_cli_args = { 0, sizeof(char*), NULL };
#define cli_args al_cli_args
typedef struct { al_vec keys; al_vec vals; } al_map;
al_vec al_push(al_vec* v, size_t es, void* elem) {
  if (v->elem_size == 0) v->elem_size = es;
  v->data = realloc(v->data, (v->len + 1) * v->elem_size);
  if (v->data && elem) memcpy((char*)v->data + v->len * v->elem_size, elem, v->elem_size);
  v->len++;
  return *v;
}
int al_chars_len(const char* s) { int n = 0; for (; *s; s++) { if ((*s & 0xC0) != 0x80) n++; } return n; }
void* al_vec_get(al_vec* vec, long idx) { if (idx < 0 || (size_t)idx >= vec->len) return NULL; return (void*)((char*)vec->data + idx * vec->elem_size); }
const char* al_char_off(const char* s, long i) { while (i > 0 && *s) { s++; while ((*s & 0xC0) == 0x80) s++; i--; } return s; }
char* al_char_at(const char* s, long i) { const char* p = al_char_off(s, i); long n = 1; while ((p[n] & 0xC0) == 0x80) n++; char* r = (char*)malloc(n + 1); memcpy(r, p, n); r[n] = 0; return r; }
char* al_substr_ch(const char* s, long a, long b) { if (a < 0) a = 0; const char* p = al_char_off(s, a); const char* q = al_char_off(p, b - a); long n = q - p; char* r = (char*)malloc(n + 1); memcpy(r, p, n); r[n] = 0; return r; }
char* al_substr(const char* s, size_t a, size_t b) {
  size_t la = strlen(s);
  if (a > la) a = la;
  if (b > la) b = la;
  if (b < a) b = a;
  char* r = (char*)malloc(b - a + 1);
  memcpy(r, s + a, b - a);
  r[b - a] = 0;
  return r;
}
char* al_strcat(const char* a, const char* b) {
  size_t la = strlen(a), lb = strlen(b);
  char* r = (char*)malloc(la + lb + 1);
  if (!r) return NULL;
  memcpy(r, a, la);
  memcpy(r + la, b, lb + 1);
  return r;
}
char* al_strdup_lit(char* s) { size_t n = strlen(s) + 1; char* r = (char*)malloc(n); memcpy(r, s, n); return r; }
static int al_vec_eq(al_vec x, al_vec y) { if (x.len != y.len || x.elem_size != y.elem_size) { return 0; } if (x.len == 0) { return 1; } return memcmp(x.data, y.data, x.len * x.elem_size) == 0; }
char* al_strcat_own(char* a, char* b) { size_t la = strlen(a), lb = strlen(b); size_t cap = la + lb + 1; if (cap < la * 2) { cap = la * 2; } char* r = (char*)realloc(a, cap); if (!r) { r = (char*)malloc(cap); memcpy(r, a, la); } memcpy(r + la, b, lb + 1); return r; }
al_vec al_vec_clone(al_vec v) { al_vec r = { v.len, v.elem_size, NULL }; if (v.len > 0 && v.data) { r.data = malloc(v.len * v.elem_size); memcpy(r.data, v.data, v.len * v.elem_size); } return r; }
al_vec al_zero_vec() { return (al_vec){ 0, 0, NULL }; }
al_map al_map_clone(al_map m) { al_map r; r.keys = al_vec_clone(m.keys); r.vals = al_vec_clone(m.vals); return r; }
al_map al_zero_map() { return (al_map){ { 0, 0, NULL }, { 0, 0, NULL } }; }
int write_file(char* path, char* content) { FILE* f = fopen(path, "wb"); if (!f) return 0; size_t n = fwrite(content, 1, strlen(content), f); fclose(f); return n > 0 || strlen(content) == 0; }
int append_file(char* path, char* content) { FILE* f = fopen(path, "ab"); if (!f) return 0; size_t n = fwrite(content, 1, strlen(content), f); fclose(f); return 1; }
int file_exists(char* path) { FILE* f = fopen(path, "rb"); if (f) { fclose(f); return 1; } return 0; }
long al_map_find(al_map* m, const char* k) { for (size_t i = 0; i < m->keys.len; i++) { char* kk = ((char**)m->keys.data)[i]; if (strcmp(kk, k) == 0) return (long)i; } return -1; }
void al_map_set_i(al_map* m, const char* k, int v) { long i = al_map_find(m, k); if (i >= 0) { ((long*)m->vals.data)[i] = v; return; } al_push(&m->keys, sizeof(char*), (void*)&k); al_push(&m->vals, sizeof(long), (void*)&v); }
void al_map_set_f(al_map* m, const char* k, double v) { long i = al_map_find(m, k); if (i >= 0) { ((double*)m->vals.data)[i] = v; return; } al_push(&m->keys, sizeof(char*), (void*)&k); al_push(&m->vals, sizeof(double), (void*)&v); }
void al_map_set_s(al_map* m, const char* k, char* v) { long i = al_map_find(m, k); if (i >= 0) { ((char**)m->vals.data)[i] = al_strdup_lit(v); return; } al_push(&m->keys, sizeof(char*), (void*)&k); al_push(&m->vals, sizeof(char*), (void*)&v); }
int al_map_get_i(al_map m, const char* k) { long i = al_map_find(&m, k); if (i < 0) { fprintf(stderr, "Map 中不存在键: %s\n", k); abort(); } return ((long*)m.vals.data)[i]; }
double al_map_get_f(al_map m, const char* k) { long i = al_map_find(&m, k); if (i < 0) { fprintf(stderr, "Map 中不存在键: %s\n", k); abort(); } return ((double*)m.vals.data)[i]; }
char* al_map_get_s(al_map m, const char* k) { long i = al_map_find(&m, k); if (i < 0) { fprintf(stderr, "Map 中不存在键: %s\n", k); abort(); } return ((char**)m.vals.data)[i]; }
void al_assert(int cond, char* msg) { if (!cond) { fprintf(stderr, "断言失败: %s\n", msg); abort(); } }
#include <time.h>
long long now_ms(void) { struct timespec ts; clock_gettime(CLOCK_REALTIME, &ts); return (long long)ts.tv_sec * 1000 + ts.tv_nsec / 1000000; }
long codepoint_at(char* s, long idx) {
  long i = 0; long cp = 0;
  while (s[i] && i <= idx) {
    unsigned char c = (unsigned char)s[i];
    if (c < 0x80) { cp = c; i += 1; }
    else if ((c & 0xE0) == 0xC0) { cp = (c & 0x1F); i += 2; }
    else if ((c & 0xF0) == 0xE0) { cp = (c & 0x0F); i += 3; }
    else if ((c & 0xF8) == 0xF0) { cp = (c & 0x07); i += 4; }
    else { cp = c; i += 1; }
    if (i > idx) break;
  }
  return cp;
}
long* read_bytes(char* path, long* out_len) {
  FILE* f = fopen(path, "rb");
  if (!f) { *out_len = -1; return NULL; }
  fseek(f, 0, SEEK_END); long n = ftell(f); fseek(f, 0, SEEK_SET);
  char* buf = (char*)malloc((size_t)n + 1);
  if (n > 0 && buf) fread(buf, 1, (size_t)n, f);
  fclose(f);
  long* result = (long*)malloc((size_t)(n + 1) * sizeof(long));
  for (long i = 0; i < n; i++) result[i] = (unsigned char)buf[i];
  if (buf) free(buf);
  *out_len = n;
  return result;
}
int write_bytes(char* path, long* data, long n) {
  FILE* f = fopen(path, "wb");
  if (!f) return 0;
  for (long i = 0; i < n; i++) fputc((int)data[i], f);
  fclose(f);
  return 1;
}
char* read_file(char* path) {
  FILE* f = fopen(path, "rb");
  if (!f) return "[read_file error: cannot open]";
  fseek(f, 0, SEEK_END);
  long n = ftell(f);
  fseek(f, 0, SEEK_SET);
  char* buf = (char*)malloc((size_t)n + 1);
  if (n > 0 && buf) fread(buf, 1, (size_t)n, f);
  if (buf) buf[n] = 0;
  fclose(f);
  return buf ? buf : "";
}
char* al_json_str(char* s, size_t* pos) {
  size_t i = *pos; while (s[i] == 32 || s[i] == 10 || s[i] == 9 || s[i] == 13) i++;
  if (s[i] != 0x22) return al_strdup_lit("");
  i++;
  size_t cap = 64; char* r = (char*)malloc(cap); size_t n = 0;
  while (s[i] && s[i] != 0x22) {
    if (s[i] == 0x5c) {
      i++;
      char c = s[i];
      if (c == 110) c = 10; else if (c == 116) c = 9; else if (c == 114) c = 13;
      if (n + 2 > cap) { cap *= 2; r = (char*)realloc(r, cap); }
      r[n++] = c; i++;
    } else {
      if (n + 2 > cap) { cap *= 2; r = (char*)realloc(r, cap); }
      r[n++] = s[i]; i++;
    }
  }
  if (s[i] == 0x22) i++;
  r[n] = 0; *pos = i; return r;
}
al_map json_decode(char* s) {
  al_map m = ((al_map){{ 0, 0, NULL }, { 0, 0, NULL }});
  size_t i = 0;
  while (s[i] == 32 || s[i] == 10 || s[i] == 9 || s[i] == 13) i++;
  if (s[i] != 0x7b) return m;
  i++;
  for (;;) {
    while (s[i] == 32 || s[i] == 10 || s[i] == 9 || s[i] == 13 || s[i] == 0x2c) i++;
    if (s[i] == 0x7d || !s[i]) break;
    if (s[i] != 0x22) break;
    char* k = al_json_str(s, &i);
    while (s[i] == 32 || s[i] == 10 || s[i] == 9) i++;
    if (s[i] == 0x3a) i++;
    while (s[i] == 32 || s[i] == 10 || s[i] == 9) i++;
    char* v;
    if (s[i] == 0x22) { v = al_json_str(s, &i); }
    else if (s[i] == 0x7b || s[i] == 0x5b) {
      char op = s[i]; char cl = (op == 0x7b) ? 0x7d : 0x5d;
      int depth = 0; size_t st = i;
      while (s[i]) { if (s[i] == op) depth++; else if (s[i] == cl) { depth--; if (!depth) { i++; break; } } i++; }
      size_t n = i - st; v = (char*)malloc(n + 1); memcpy(v, s + st, n); v[n] = 0;
    }
    else { size_t st = i; while (s[i] && s[i] != 0x2c && s[i] != 0x7d) i++; size_t n = i - st; v = (char*)malloc(n + 1); memcpy(v, s + st, n); v[n] = 0; }
    al_push(&m.keys, sizeof(char*), &k);
    al_push(&m.vals, sizeof(char*), &v);
  }
  return m;
}
char* read_line(void) {
  size_t cap = 256; char* buf = (char*)malloc(cap); size_t n = 0;
  int c;
  while ((c = fgetc(stdin)) != EOF) {
    if (c == 10) break;
    if (c == 13) continue;
    if (n + 2 > cap) { cap *= 2; buf = (char*)realloc(buf, cap); }
    buf[n++] = (char)c;
  }
  buf[n] = 0;
  return buf;
}
static char* al_json_esc(char* s) {
  size_t n = strlen(s); char* r = (char*)malloc(n*6+3); char* w = r;
  *w++ = 0x22;
  for (size_t i=0;i<n;i++) { unsigned char c=(unsigned char)s[i];
    if (c==0x22){*w++=0x5c;*w++=0x22;}
    else if (c==0x5c){*w++=0x5c;*w++=0x5c;}
    else if (c==10){*w++=0x5c;*w++=110;}
    else if (c==13){*w++=0x5c;*w++=114;}
    else if (c==9){*w++=0x5c;*w++=116;}
    else {*w++=(char)c;}
  }
  *w++=0x22; *w=0; return r;
}
char* json_encode(al_map m) {
  char* r = (char*)malloc(m.keys.len*64+16); char* w = r;
  *w++ = 0x7b;
  for (size_t i=0;i<m.keys.len;i++) {
    if (i) *w++ = 0x2c;
    char* k = ((char**)m.keys.data)[i];
    char* v = ((char**)m.vals.data)[i];
    char* ek = al_json_esc(k); size_t kl = strlen(ek); memcpy(w, ek, kl); w += kl;
    *w++ = 0x3a;
    if (v && ((unsigned char)v[0]==0x7b || (unsigned char)v[0]==0x5b || strncmp(v,"true",4)==0 || strncmp(v,"false",5)==0 || strncmp(v,"null",4)==0)) { size_t vl=strlen(v); memcpy(w,v,vl); w+=vl; }
    else if (v && strspn(v,"-0123456789.eE+")==strlen(v) && strlen(v)>0) { size_t vl=strlen(v); memcpy(w,v,vl); w+=vl; }
    else { char* ev = al_json_esc(v?v:""); size_t vl=strlen(ev); memcpy(w,ev,vl); w+=vl; }
  }
  *w++ = 0x7d; *w=0; return r;
}
char* env(char* name) { char* v = getenv(name); return al_strdup_lit(v ? v : ""); }
#include <windows.h>

static HWND al_ui_h[64]; static void (*al_ui_cb[64])(void); static int al_ui_n = 0; static int al_ui_cls = 0; static HINSTANCE al_ui_inst = NULL;
static wchar_t* al_utf16(const char* s) { int n = MultiByteToWideChar(CP_UTF8, 0, s, -1, NULL, 0); wchar_t* w = (wchar_t*)malloc(n * 2); MultiByteToWideChar(CP_UTF8, 0, s, -1, w, n); return w; }
static LRESULT CALLBACK al_ui_proc(HWND h, UINT m, WPARAM w, LPARAM l) {
  if (m == WM_COMMAND && HIWORD(w) == BN_CLICKED) { int id = LOWORD(w); if (id >= 0 && id < 64 && al_ui_cb[id]) al_ui_cb[id](); return 0; }
  if (m == WM_DESTROY) { PostQuitMessage(0); return 0; }
  return DefWindowProcW(h, m, w, l);
}
static int window_new(char* title, int wd, int ht) {
  if (!al_ui_cls) { WNDCLASSW wc; memset(&wc, 0, sizeof(wc)); wc.lpfnWndProc = al_ui_proc; wc.hInstance = GetModuleHandle(NULL); wc.lpszClassName = L"AineWin"; wc.hCursor = LoadCursor(NULL, IDC_ARROW); RegisterClassW(&wc); al_ui_cls = 1; al_ui_inst = GetModuleHandle(NULL); }
  HWND h = CreateWindowExW(0, L"AineWin", al_utf16(title), WS_OVERLAPPEDWINDOW, 100, 100, wd, ht, NULL, NULL, al_ui_inst, NULL);
  al_ui_h[al_ui_n] = h; al_ui_n++; return al_ui_n - 1;
}
static int al_ui_child(HWND parent, wchar_t* cls, char* text, int x, int y, int w2, int h2, int id) {
  HWND h = CreateWindowExW(0, cls, al_utf16(text), WS_CHILD | WS_VISIBLE, x, y, w2, h2, parent, (HMENU)(INT_PTR)id, al_ui_inst, NULL);
  al_ui_h[id] = h; if (id >= al_ui_n) al_ui_n = id + 1; return id;
}
#include <d2d1.h>
#include <dwrite.h>
static ID2D1Factory* g_d2d = NULL;
static ID2D1HwndRenderTarget* g_rt = NULL;
static IDWriteFactory* g_dw = NULL;
static IDWriteTextFormat* g_tf_ui = NULL;
static IDWriteTextFormat* g_tf_code = NULL;
static const IID al_IID_IDWriteFactory = {0xb859ee5a,0xd838,0x4b5b,{0xa2,0xe8,0x1a,0xdc,0x7d,0x93,0xdb,0x48}};
enum { AL_PANEL, AL_LABEL, AL_BUTTON, AL_LIST, AL_SRECT, AL_HLINE };
#define AL_MAXC 2048
typedef struct { int type; float x, y, w, h; float rad; float size; int mono; char text[256]; D2D1_COLOR_F bg; D2D1_COLOR_F fg; void (*cb)(void); } al_comp;
static al_comp g_comps[AL_MAXC]; static int g_n = 0; static int g_pressed = -1;
typedef struct { float x, y, w, h; int id; } al_clk;
static al_clk g_clicks[256]; static int g_nc = 0; static int g_last_click = -1; static int g_click_flag = 0; static int g_resize_flag = 0;
static IDWriteTextFormat* al_tfs[40]; static float al_tfs_sz[40]; static int al_tfs_mo[40]; static int al_tfs_n = 0;
typedef struct { NMHDR hdr; int position; } AlSciPos;
static void (*g_style_cb)(int) = NULL;
static D2D1_COLOR_F al_col(int r, int g, int b) { D2D1_COLOR_F c; c.r = r / 255.0f; c.g = g / 255.0f; c.b = b / 255.0f; c.a = 1.0f; return c; }
static D2D1_COLOR_F al_theme(const char* n) {
  if (n[0] == 0) return al_col(251, 250, 247);
  if (n[0] == 112 && n[1] == 97) return al_col(246, 244, 239);
  if (n[0] == 108) return al_col(223, 221, 213);
  if (n[0] == 103 && n[1] == 114) return al_col(66, 101, 91);
  if (n[0] == 103 && n[1] == 50) return al_col(230, 236, 233);
  if (n[0] == 105) return al_col(37, 37, 31);
  if (n[0] == 115) return al_col(119, 118, 111);
  if (n[0] == 116) return al_col(242, 239, 233);
  return al_col(251, 250, 247);
}
static D2D1_COLOR_F al_rgb(int v) { D2D1_COLOR_F c; c.r = ((v >> 16) & 255) / 255.0f; c.g = ((v >> 8) & 255) / 255.0f; c.b = (v & 255) / 255.0f; c.a = 1.0f; return c; }
static IDWriteTextFormat* al_tf(float sz, int mono) {
  for (int i = 0; i < al_tfs_n; i++) { if (al_tfs_sz[i] == sz && al_tfs_mo[i] == mono && al_tfs[i]) return al_tfs[i]; }
  IDWriteTextFormat* tf = NULL;
  g_dw->lpVtbl->CreateTextFormat(g_dw, mono ? L"Consolas" : L"Segoe UI", NULL, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL, sz, L"", &tf);
  if (al_tfs_n < 40) { al_tfs[al_tfs_n] = tf; al_tfs_sz[al_tfs_n] = sz; al_tfs_mo[al_tfs_n] = mono; al_tfs_n++; }
  return tf;
}
static wchar_t al_w[512];
static LRESULT CALLBACK al_rt_proc(HWND h, UINT m, WPARAM w, LPARAM l) {
  if (m == WM_PAINT && g_rt) { PAINTSTRUCT ps; BeginPaint(h, &ps);
    ID2D1HwndRenderTarget_BeginDraw(g_rt);
    D2D1_COLOR_F paper = al_theme("paper");
    ID2D1HwndRenderTarget_Clear(g_rt, &paper);
    for (int i = 0; i < g_n; i++) {
      al_comp* cp = &g_comps[i];
      D2D1_RECT_F r; r.left = cp->x; r.top = cp->y; r.right = cp->x + cp->w; r.bottom = cp->y + cp->h;
      if (cp->type == AL_PANEL) {
        ID2D1SolidColorBrush* b = NULL;
        ID2D1HwndRenderTarget_CreateSolidColorBrush(g_rt, &cp->bg, NULL, &b);
        if (b) { D2D1_ROUNDED_RECT rr; rr.rect = r; rr.radiusX = cp->rad; rr.radiusY = cp->rad; ID2D1RenderTarget_FillRoundedRectangle((ID2D1RenderTarget*)g_rt, &rr, (ID2D1Brush*)b); ID2D1SolidColorBrush_Release(b); }
      } else if (cp->type == AL_HLINE) {
        ID2D1SolidColorBrush* b = NULL;
        ID2D1HwndRenderTarget_CreateSolidColorBrush(g_rt, &cp->bg, NULL, &b);
        if (b) { D2D1_POINT_2F p0, p1; p0.x = cp->x; p0.y = cp->y; p1.x = cp->x + cp->w; p1.y = cp->y; ID2D1RenderTarget_DrawLine((ID2D1RenderTarget*)g_rt, p0, p1, (ID2D1Brush*)b, 1.0f, NULL); ID2D1SolidColorBrush_Release(b); }
      } else if (cp->type == AL_SRECT) {
        ID2D1SolidColorBrush* b = NULL;
        ID2D1HwndRenderTarget_CreateSolidColorBrush(g_rt, &cp->bg, NULL, &b);
        if (b) { D2D1_ROUNDED_RECT rr; rr.rect = r; rr.radiusX = cp->rad; rr.radiusY = cp->rad; ID2D1RenderTarget_DrawRoundedRectangle((ID2D1RenderTarget*)g_rt, &rr, (ID2D1Brush*)b, 1.0f, NULL); ID2D1SolidColorBrush_Release(b); }
      } else if (cp->type == AL_BUTTON) {
        D2D1_COLOR_F bg2 = (g_pressed == i) ? al_theme("green2") : al_theme("panel2");
        ID2D1SolidColorBrush* b = NULL;
        ID2D1HwndRenderTarget_CreateSolidColorBrush(g_rt, &bg2, NULL, &b);
        if (b) { D2D1_ROUNDED_RECT rr; rr.rect = r; rr.radiusX = cp->rad; rr.radiusY = cp->rad; ID2D1RenderTarget_FillRoundedRectangle((ID2D1RenderTarget*)g_rt, &rr, (ID2D1Brush*)b); ID2D1SolidColorBrush_Release(b); }
        b = NULL;
        ID2D1HwndRenderTarget_CreateSolidColorBrush(g_rt, &cp->bg, NULL, &b);
        if (b) { ID2D1RenderTarget_DrawRectangle((ID2D1RenderTarget*)g_rt, &r, (ID2D1Brush*)b, 1.0f, NULL); ID2D1SolidColorBrush_Release(b); }
        UINT32 ln = 0; wchar_t* wv = al_utf16(cp->text); while (wv[ln]) ln++;
        b = NULL;
        D2D1_COLOR_F grn = al_theme("green");
        ID2D1HwndRenderTarget_CreateSolidColorBrush(g_rt, &grn, NULL, &b);
        if (b) { ID2D1RenderTarget_DrawText((ID2D1RenderTarget*)g_rt, wv, ln, g_tf_ui, &r, (ID2D1Brush*)b, D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL); ID2D1SolidColorBrush_Release(b); }
      } else if (cp->type == AL_LIST) {
        ID2D1SolidColorBrush* b = NULL;
        ID2D1HwndRenderTarget_CreateSolidColorBrush(g_rt, &cp->bg, NULL, &b);
        if (b) { D2D1_ROUNDED_RECT rr; rr.rect = r; rr.radiusX = cp->rad; rr.radiusY = cp->rad; ID2D1RenderTarget_FillRoundedRectangle((ID2D1RenderTarget*)g_rt, &rr, (ID2D1Brush*)b); ID2D1SolidColorBrush_Release(b); }
      } else if (cp->type == AL_LABEL) {
        UINT32 ln = 0; wchar_t* wv = al_utf16(cp->text); while (wv[ln]) ln++;
        IDWriteTextFormat* tfv = (cp->size > 0) ? al_tf(cp->size, cp->mono) : g_tf_ui;
        if (tfv) { ID2D1SolidColorBrush* b = NULL;
        ID2D1HwndRenderTarget_CreateSolidColorBrush(g_rt, &cp->fg, NULL, &b);
        if (b) { ID2D1RenderTarget_DrawText((ID2D1RenderTarget*)g_rt, wv, ln, tfv, &r, (ID2D1Brush*)b, D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL); ID2D1SolidColorBrush_Release(b); } }
      }
    }
    ID2D1HwndRenderTarget_EndDraw(g_rt, NULL, NULL);
    EndPaint(h, &ps); return 0; }
  if (m == WM_SIZE && g_rt) { D2D1_SIZE_U sz2; sz2.width = (UINT32)LOWORD(l); sz2.height = (UINT32)HIWORD(l); ID2D1HwndRenderTarget_Resize(g_rt, &sz2); g_resize_flag = 1; InvalidateRect(h, NULL, FALSE); return 0; }
  if (m == WM_KEYDOWN || m == WM_SYSKEYDOWN) {
    int vk = (int)w;
    if (vk == 116) { g_last_click = 205; g_click_flag = 1; return 0; }
    if (vk == 117) { g_last_click = 220; g_click_flag = 1; return 0; }
    if (vk == 118) { g_last_click = 230; g_click_flag = 1; return 0; }
    if (vk == 119) { g_last_click = 200; g_click_flag = 1; return 0; }
  }
  if (m == WM_LBUTTONDOWN) { float mx = (float)(short)LOWORD(l), my = (float)(short)HIWORD(l);
    for (int i = 0; i < g_n; i++) { al_comp* cp = &g_comps[i];
      if (cp->type == AL_BUTTON && mx >= cp->x && mx <= cp->x + cp->w && my >= cp->y && my <= cp->y + cp->h) { g_pressed = i; if (cp->cb) cp->cb(); InvalidateRect(h, NULL, FALSE); return 0; } }
    for (int i = 0; i < g_nc; i++) { if (mx >= g_clicks[i].x && mx <= g_clicks[i].x + g_clicks[i].w && my >= g_clicks[i].y && my <= g_clicks[i].y + g_clicks[i].h) { g_last_click = g_clicks[i].id; g_click_flag = 1; return 0; } }
    return 0; }
  if (m == WM_LBUTTONUP) { g_pressed = -1; InvalidateRect(h, NULL, FALSE); return 0; }
  if (m == WM_NOTIFY) {
    typedef struct { NMHDR hdr; int position; int ch; int modifiers; } AlSciKey;
    AlSciKey* sk = (AlSciKey*)l;
    if (sk && sk->hdr.code == 2000 && g_style_cb) { g_style_cb(sk->position); return 0; }
    if (sk && sk->hdr.code == 2005) {
      int ch = sk->ch;
      if (ch == 116) { g_last_click = 205; g_click_flag = 1; return 0; }
      if (ch == 117) { g_last_click = 220; g_click_flag = 1; return 0; }
      if (ch == 118) { g_last_click = 230; g_click_flag = 1; return 0; }
      if (ch == 119) { g_last_click = 200; g_click_flag = 1; return 0; }
    }
  }
  if (m == 0x0084) { return 1; }
  if (m == WM_DESTROY) { PostQuitMessage(0); return 0; }
  return DefWindowProcW(h, m, w, l); }
static int rt_window(char* title, int wd, int ht) {
  D2D1_FACTORY_OPTIONS fo; memset(&fo, 0, sizeof(fo));
  if (FAILED(D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, &IID_ID2D1Factory, &fo, (void**)&g_d2d))) return -1;
  if (FAILED(DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED, &al_IID_IDWriteFactory, (IUnknown**)&g_dw))) return -1;
  g_dw->lpVtbl->CreateTextFormat(g_dw, L"Segoe UI", NULL, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL, 12.0f, L"", &g_tf_ui);
  g_dw->lpVtbl->CreateTextFormat(g_dw, L"Consolas", NULL, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL, 13.0f, L"", &g_tf_code);
  WNDCLASSW wc; memset(&wc, 0, sizeof(wc)); wc.lpfnWndProc = al_rt_proc; wc.hInstance = GetModuleHandle(NULL); wc.lpszClassName = L"AineRT"; wc.hCursor = LoadCursor(NULL, IDC_ARROW); RegisterClassW(&wc);
  HWND h = CreateWindowExW(0, L"AineRT", al_utf16(title), WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN, 60, 60, wd, ht, NULL, NULL, GetModuleHandle(NULL), NULL);
  RECT rc2; GetClientRect(h, &rc2);
  D2D1_SIZE_U sz2; sz2.width = (UINT32)(rc2.right - rc2.left); sz2.height = (UINT32)(rc2.bottom - rc2.top);
  D2D1_RENDER_TARGET_PROPERTIES rtp; memset(&rtp, 0, sizeof(rtp)); rtp.type = D2D1_RENDER_TARGET_TYPE_DEFAULT; rtp.pixelFormat.format = DXGI_FORMAT_UNKNOWN; rtp.pixelFormat.alphaMode = D2D1_ALPHA_MODE_PREMULTIPLIED;
  D2D1_HWND_RENDER_TARGET_PROPERTIES hp; hp.hwnd = h; hp.pixelSize = sz2; hp.presentOptions = D2D1_PRESENT_OPTIONS_NONE;
  if (FAILED(ID2D1Factory_CreateHwndRenderTarget(g_d2d, &rtp, &hp, &g_rt))) return -1;
  ShowWindow(h, SW_SHOW);
  return 0;
}
static int rt_comp(int type, float x, float y, float w2, float h2, const char* text, const char* bgname, const char* fgname, void (*cb)(void)) {
  if (g_n >= AL_MAXC) return -1;
  al_comp* cp = &g_comps[g_n]; cp->type = type; cp->x = x; cp->y = y; cp->w = w2; cp->h = h2; cp->rad = (type == AL_PANEL) ? 10 : 8; cp->size = 0; cp->mono = 0;
  size_t tl = strlen(text); if (tl > 255) tl = 255; memcpy(cp->text, text, tl); cp->text[tl] = 0;
  cp->bg = al_theme(bgname); cp->fg = al_theme(fgname); cp->cb = cb; g_n++; return g_n - 1;
}
static void rt_set_comp_text(int id, const char* text) { if (id >= 0 && id < g_n) { size_t tl = strlen(text); if (tl > 255) tl = 255; memcpy(g_comps[id].text, text, tl); g_comps[id].text[tl] = 0; } }
static int rt_put(int type, float x, float y, float w2, float h2, float rad, const char* text, int rgb, int mono, float sz) {
  if (g_n >= AL_MAXC) return -1;
  al_comp* cp = &g_comps[g_n]; cp->type = type; cp->x = x; cp->y = y; cp->w = w2; cp->h = h2; cp->rad = rad; cp->size = sz; cp->mono = mono;
  size_t tl = strlen(text); if (tl > 255) tl = 255; memcpy(cp->text, text, tl); cp->text[tl] = 0;
  cp->fg = al_rgb(rgb); cp->bg = al_rgb(rgb); cp->cb = NULL; g_n++; return g_n - 1;
}
static void rt_begin(void) { g_n = 0; g_nc = 0; g_click_flag = 0; }
static int rt_fill(float x, float y, float w2, float h2, int rgb) { return rt_put(AL_PANEL, x, y, w2, h2, 0, "", rgb, 0, 0); }
static int rt_rrect(float x, float y, float w2, float h2, float rad, int rgb) { return rt_put(AL_PANEL, x, y, w2, h2, rad, "", rgb, 0, 0); }
static int rt_srect(float x, float y, float w2, float h2, float rad, int rgb) { return rt_put(AL_SRECT, x, y, w2, h2, rad, "", rgb, 0, 0); }
static int rt_hline(float x0, float x1, float y, int rgb) { return rt_put(AL_HLINE, x0, y, x1 - x0, 0, 0, "", rgb, 0, 0); }
static int rt_text(float x, float y, const char* s, int rgb, int mono, float sz) { return rt_put(AL_LABEL, x, y, 2000, 60, 0, s, rgb, mono, sz); }
static int rt_click(float x, float y, float w2, float h2, int id) { if (g_nc >= 256) return -1; g_clicks[g_nc].x = x; g_clicks[g_nc].y = y; g_clicks[g_nc].w = w2; g_clicks[g_nc].h = h2; g_clicks[g_nc].id = id; g_nc++; return g_nc - 1; }
static int rt_wait_click(void) { MSG msg; g_click_flag = 0; while (GetMessage(&msg, NULL, 0, 0) > 0) { TranslateMessage(&msg); DispatchMessage(&msg); if (g_click_flag) { g_click_flag = 0; return g_last_click; } } return -2; }
static HWND al_rt_hwnd(void) { return g_rt ? (HWND)ID2D1HwndRenderTarget_GetHwnd(g_rt) : NULL; }
static int rt_w(void) { RECT rc; HWND h = al_rt_hwnd(); if (!h) return 1200; GetClientRect(h, &rc); return rc.right - rc.left; }
static int rt_h(void) { RECT rc; HWND h = al_rt_hwnd(); if (!h) return 760; GetClientRect(h, &rc); return rc.bottom - rc.top; }
static int rt_wait_event(void) { MSG msg; g_click_flag = 0; while (GetMessage(&msg, NULL, 0, 0) > 0) { TranslateMessage(&msg); DispatchMessage(&msg); if (g_click_flag) { g_click_flag = 0; return g_last_click; } if (g_resize_flag) { g_resize_flag = 0; return -1; } } return -2; }
static void rt_invalidate(void) { HWND h = g_rt ? (HWND)ID2D1HwndRenderTarget_GetHwnd(g_rt) : NULL; if (h) InvalidateRect(h, NULL, FALSE); }
static void rt_run(void) { HWND h = g_rt ? (HWND)ID2D1HwndRenderTarget_GetHwnd(g_rt) : NULL; if (h) ShowWindow(h, SW_SHOW); MSG msg; while (GetMessage(&msg, NULL, 0, 0) > 0) { TranslateMessage(&msg); DispatchMessage(&msg); } }
static int label_new(int win, char* text, int x, int y) { return al_ui_child(al_ui_h[win], L"STATIC", text, x, y, 220, 26, al_ui_n); }
static int button_new(int win, char* text, int x, int y) { return al_ui_child(al_ui_h[win], L"BUTTON", text, x, y, 96, 30, al_ui_n); }
static int input_new(int win, char* text, int x, int y) { return al_ui_child(al_ui_h[win], L"EDIT", text, x, y, 220, 26, al_ui_n); }
static int listbox_new(int win, int x, int y, int w2, int h2) { HWND h = CreateWindowExW(0, L"LISTBOX", L"", WS_CHILD | WS_VISIBLE | WS_VSCROLL | LBS_NOTIFY, x, y, w2, h2, al_ui_h[win], (HMENU)(INT_PTR)al_ui_n, al_ui_inst, NULL); al_ui_h[al_ui_n] = h; al_ui_n++; return al_ui_n - 1; }
static void listbox_add(int id, char* text) { if (id >= 0 && id < 64 && al_ui_h[id]) SendMessageW(al_ui_h[id], LB_ADDSTRING, 0, (LPARAM)al_utf16(text)); }
static void listbox_clear(int id) { if (id >= 0 && id < 64 && al_ui_h[id]) SendMessageW(al_ui_h[id], LB_RESETCONTENT, 0, 0); }
static int listbox_selected(int id) { if (id >= 0 && id < 64 && al_ui_h[id]) return (int)SendMessageW(al_ui_h[id], LB_GETCURSEL, 0, 0); return -1; }
static void ui_run(void) { for (int i = 0; i < al_ui_n; i++) ShowWindow(al_ui_h[i], SW_SHOW); MSG msg; while (GetMessage(&msg, NULL, 0, 0) > 0) { TranslateMessage(&msg); DispatchMessage(&msg); } }
static void set_text(int id, char* text) { if (id >= 0 && id < 64 && al_ui_h[id]) SetWindowTextW(al_ui_h[id], al_utf16(text)); }
static char* get_text(int id) { wchar_t w[512]; if (id >= 0 && id < 64 && al_ui_h[id]) GetWindowTextW(al_ui_h[id], w, 512); else w[0] = 0; int n = WideCharToMultiByte(CP_UTF8, 0, w, -1, NULL, 0, NULL, NULL); char* r = (char*)malloc(n); WideCharToMultiByte(CP_UTF8, 0, w, -1, r, n, NULL, NULL); return r; }
static void button_onclick(int id, void (*fn)(void)) { if (id >= 0 && id < 64) al_ui_cb[id] = fn; }
static int edit_new(int win, char* text, int x, int y, int w2, int h2) { HWND h = CreateWindowExW(0, L"EDIT", al_utf16(text), WS_CHILD | WS_VISIBLE | WS_VSCROLL | WS_HSCROLL | ES_MULTILINE | ES_AUTOVSCROLL | ES_AUTOHSCROLL | WS_BORDER, x, y, w2, h2, al_ui_h[win], (HMENU)(INT_PTR)al_ui_n, GetModuleHandle(NULL), NULL); al_ui_h[al_ui_n] = h; al_ui_n++; return al_ui_n - 1; }
static void listbox_ondblclick(int id, void (*fn)(void)) { if (id >= 0 && id < 64) al_ui_cb[id] = fn; }
static char* listbox_get(int id, int idx) { wchar_t w[512]; if (id >= 0 && id < 64 && al_ui_h[id]) SendMessageW(al_ui_h[id], LB_GETTEXT, (WPARAM)idx, (LPARAM)w); else w[0] = 0; int n = WideCharToMultiByte(CP_UTF8, 0, w, -1, NULL, 0, NULL, NULL); char* r = (char*)malloc(n); WideCharToMultiByte(CP_UTF8, 0, w, -1, r, n, NULL, NULL); return r; }
static HMODULE g_sci_dll = NULL; static HWND g_sci = NULL;
static int sci_init(void) {
  HMODULE sc = LoadLibraryA("bin/Scintilla.dll"); if (!sc) sc = LoadLibraryA("Scintilla.dll");
  if (!sc) return -1;
  int (*reg)(void*) = (int (*)(void*))GetProcAddress(sc, "Scintilla_RegisterClasses");
  if (reg) reg(GetModuleHandle(NULL));
  return 0;
}
static int sci_new(int x, int y, int w2, int h2) {
  HWND parent = al_rt_hwnd(); if (!parent) return -1;
  g_sci = CreateWindowExW(0, L"Scintilla", L"", WS_CHILD | WS_VISIBLE | WS_VSCROLL | WS_HSCROLL, x, y, w2, h2, parent, NULL, GetModuleHandle(NULL), NULL);
  if (!g_sci) return -1;
  SendMessageW(g_sci, 4003, 0, (LPARAM)0);
  return 0;
}
static int sci_msg(unsigned int msg, unsigned int wp, int lp) { if (!g_sci) return 0; return (int)SendMessageW(g_sci, msg, (WPARAM)wp, (LPARAM)(intptr_t)lp); }
static int sci_msgp(unsigned int msg, unsigned int wp, const char* lp) { if (!g_sci) return 0; return (int)SendMessageW(g_sci, msg, (WPARAM)wp, (LPARAM)lp); }
static void rt_on_style(void (*fn)(int)) { g_style_cb = fn; }
static char* sci_get_line(int line) { if (!g_sci) return al_strdup_lit(""); int n = (int)SendMessageW(g_sci, 2350, (WPARAM)line, 0); char* buf = (char*)malloc(n + 2); if (!buf) return al_strdup_lit(""); SendMessageW(g_sci, 2153, (WPARAM)line, (LPARAM)buf); buf[n] = 0; return buf; }
static void sci_set_text(char* s) { if (!g_sci || !s) return; SendMessageW(g_sci, 2004, 0, 0); int n = 0; while (s[n]) n++; SendMessageW(g_sci, 2001, (WPARAM)n, (LPARAM)s); }
static char* sci_get_text(void) { if (!g_sci) return al_strdup_lit(""); int n = (int)SendMessageW(g_sci, 2006, 0, 0); char* buf = (char*)malloc(n + 1); if (!buf) return al_strdup_lit(""); SendMessageW(g_sci, 2182, (WPARAM)n + 1, (LPARAM)buf); buf[n] = 0; char* r2 = buf; char* w2 = buf; while (*r2) { if (*r2 == '\r') { if (r2[1] == '\n') r2++; *w2++ = '\n'; r2++; } else { *w2++ = *r2++; } } *w2 = 0; return buf; }
static void sci_fit(int x, int y, int w2, int h2) { if (g_sci) MoveWindow(g_sci, x, y, w2, h2, TRUE); }
static int sys_exec(char* cmd) { STARTUPINFOA si; PROCESS_INFORMATION pi; memset(&si, 0, sizeof(si)); si.cb = sizeof(si); si.dwFlags = STARTF_USESHOWWINDOW; si.wShowWindow = SW_HIDE; if (!CreateProcessA(NULL, cmd, NULL, NULL, FALSE, 0x08000000, NULL, NULL, &si, &pi)) return -1; WaitForSingleObject(pi.hProcess, 600000); DWORD code = 1; GetExitCodeProcess(pi.hProcess, &code); CloseHandle(pi.hThread); CloseHandle(pi.hProcess); return (int)code; }
static int sys_open(char* path) { HINSTANCE r = ShellExecuteA(NULL, "open", path, NULL, NULL, SW_SHOWNORMAL); return ((INT_PTR)r) > 32 ? 0 : -1; }
static al_vec file_list(char* path) {
  al_vec outv = { 0, sizeof(char*), NULL };
  char pat[1024]; snprintf(pat, sizeof(pat), "%s/*", path);
  wchar_t* wpat = al_utf16(pat);
  WIN32_FIND_DATAW fd; HANDLE fh = FindFirstFileW(wpat, &fd);
  if (fh != INVALID_HANDLE_VALUE) {
    do {
      if (wcscmp(fd.cFileName, L".") == 0 || wcscmp(fd.cFileName, L"..") == 0) continue;
      int wn = WideCharToMultiByte(CP_UTF8, 0, fd.cFileName, -1, NULL, 0, NULL, NULL);
      char* s = (char*)malloc(wn); WideCharToMultiByte(CP_UTF8, 0, fd.cFileName, -1, s, wn, NULL, NULL);
      al_push(&outv, sizeof(char*), (void*)&s);
    } while (FindNextFileW(fh, &fd));
    FindClose(fh);
  }
  for (int i = 0; i + 1 < (int)outv.len; i++) { for (int j = i + 1; j < (int)outv.len; j++) {
    char* a = ((char**)outv.data)[i]; char* b = ((char**)outv.data)[j];
    if (strcmp(a, b) > 0) { ((char**)outv.data)[i] = b; ((char**)outv.data)[j] = a; }
  } }
  return outv;
}
static void* al_dup(const void* p, size_t n) {
  void* r = malloc(n);
  if (r && p) memcpy(r, p, n);
  return r;
}
static char* al_char_to_str(char c) { char* r = (char*)malloc(2); r[0] = c; r[1] = 0; return r; }
static char* al_num(long v) { char* r = (char*)malloc(32); snprintf(r, 32, "%ld", v); return r; }
static char* al_fnum(double v) { char* r = (char*)malloc(32); if (v == (long long)v && v >= -1e15 && v <= 1e15) { snprintf(r, 32, "%.1f", v); } else { snprintf(r, 32, "%g", v); } return r; }
static char* al_bool(int b) { return b ? "true" : "false"; }
static char* al_trim(char* s) { char* r = (char*)malloc(strlen(s) + 1); strcpy(r, s); char* st = r; while (*st == ' ' || *st == '\t' || *st == '\r' || *st == '\n') st++; char* en = st + strlen(st); while (en > st && (en[-1] == ' ' || en[-1] == '\t' || en[-1] == '\r' || en[-1] == '\n')) en--; *en = 0; if (st != r) memmove(r, st, strlen(st) + 1); return r; }
static al_vec al_split(char* s, char* sep) { al_vec r = { 0, sizeof(char*), NULL }; size_t sl = strlen(sep); if (sl == 0) { for (size_t i = 0; i < strlen(s); i++) { char* p = (char*)malloc(2); p[0] = s[i]; p[1] = 0; al_push(&r, sizeof(char*), &p); } return r; } else { char* cur = s; while (1) { char* hit = strstr(cur, sep); if (!hit) { al_push(&r, sizeof(char*), &(char*){ cur }); break; } size_t n = (size_t)(hit - cur); char* p = (char*)malloc(n + 1); memcpy(p, cur, n); p[n] = 0; al_push(&r, sizeof(char*), &p); cur = hit + sl; } return r; } }
static char* al_replace(char* s, char* from, char* to) { size_t fl = strlen(from), tl = strlen(to), sl = strlen(s); if (fl == 0) return s; size_t cnt = 0; char* q = s; while ((q = strstr(q, from)) != NULL) { cnt++; q += fl; } char* r = (char*)malloc(sl + cnt * (tl - fl) + 1); char* o = r; char* p = s; while (1) { char* hit = strstr(p, from); if (!hit) { strcpy(o, p); break; } size_t pre = (size_t)(hit - p); memcpy(o, p, pre); o += pre; memcpy(o, to, tl); o += tl; p = hit + fl; } return r; }
static al_vec al_slice(al_vec v, size_t a, size_t b) {
  if (b > v.len) b = v.len;
  if (a > b) a = b;
  size_t n = b - a;
  al_vec r = { n, v.elem_size, NULL };
  if (n > 0) {
    r.data = malloc(n * v.elem_size);
    if (v.data) memcpy(r.data, (char*)v.data + a * v.elem_size, n * v.elem_size);
  }
  return r;
}
typedef struct { int tag; union { int i; double f; char* s; al_vec v; void* p;struct Severity* t_Severity;struct Diagnostic* t_Diagnostic;struct ExplicitCommand* t_ExplicitCommand;struct EscapeHit* t_EscapeHit;struct Arg* t_Arg;struct KVPair* t_KVPair;struct CommandUse* t_CommandUse;struct ParseErr* t_ParseErr;struct HeaderParsed* t_HeaderParsed;struct ValParsed* t_ValParsed;struct Balanced* t_Balanced;struct Inline* t_Inline;struct LineClass* t_LineClass;struct MarkerKind* t_MarkerKind;struct MarkerAtom* t_MarkerAtom;struct LwAtom* t_LwAtom;struct AtomOut* t_AtomOut;struct ScanOut* t_ScanOut;struct CloseSpan* t_CloseSpan;struct CloseHit* t_CloseHit;struct Frame* t_Frame;struct Block* t_Block;struct LexedBlock* t_LexedBlock;struct LexOutput* t_LexOutput;struct CmdOutcome* t_CmdOutcome;struct SourceSpan* t_SourceSpan;struct TextPatch* t_TextPatch;struct Buffer* t_Buffer;struct BufWithPatch* t_BufWithPatch; } data; } al_opt;
#define al_some_i(x) ((al_opt){ .tag = 1, .data.i = (x) })
#define al_some_f(x) ((al_opt){ .tag = 1, .data.f = (x) })
#define al_some_s(x) ((al_opt){ .tag = 1, .data.s = (x) })
#define al_some_v(x) ((al_opt){ .tag = 1, .data.v = (x) })
#define al_some_p(x) ((al_opt){ .tag = 1, .data.p = (void*)(x) })
#define al_some_t_Severity(x) __extension__ ({ struct Severity _t = (x); (al_opt){ .tag = 1, .data.t_Severity = al_dup(&_t, sizeof(struct Severity)) }; })
#define al_some_t_Diagnostic(x) __extension__ ({ struct Diagnostic _t = (x); (al_opt){ .tag = 1, .data.t_Diagnostic = al_dup(&_t, sizeof(struct Diagnostic)) }; })
#define al_some_t_ExplicitCommand(x) __extension__ ({ struct ExplicitCommand _t = (x); (al_opt){ .tag = 1, .data.t_ExplicitCommand = al_dup(&_t, sizeof(struct ExplicitCommand)) }; })
#define al_some_t_EscapeHit(x) __extension__ ({ struct EscapeHit _t = (x); (al_opt){ .tag = 1, .data.t_EscapeHit = al_dup(&_t, sizeof(struct EscapeHit)) }; })
#define al_some_t_Arg(x) __extension__ ({ struct Arg _t = (x); (al_opt){ .tag = 1, .data.t_Arg = al_dup(&_t, sizeof(struct Arg)) }; })
#define al_some_t_KVPair(x) __extension__ ({ struct KVPair _t = (x); (al_opt){ .tag = 1, .data.t_KVPair = al_dup(&_t, sizeof(struct KVPair)) }; })
#define al_some_t_CommandUse(x) __extension__ ({ struct CommandUse _t = (x); (al_opt){ .tag = 1, .data.t_CommandUse = al_dup(&_t, sizeof(struct CommandUse)) }; })
#define al_some_t_ParseErr(x) __extension__ ({ struct ParseErr _t = (x); (al_opt){ .tag = 1, .data.t_ParseErr = al_dup(&_t, sizeof(struct ParseErr)) }; })
#define al_some_t_HeaderParsed(x) __extension__ ({ struct HeaderParsed _t = (x); (al_opt){ .tag = 1, .data.t_HeaderParsed = al_dup(&_t, sizeof(struct HeaderParsed)) }; })
#define al_some_t_ValParsed(x) __extension__ ({ struct ValParsed _t = (x); (al_opt){ .tag = 1, .data.t_ValParsed = al_dup(&_t, sizeof(struct ValParsed)) }; })
#define al_some_t_Balanced(x) __extension__ ({ struct Balanced _t = (x); (al_opt){ .tag = 1, .data.t_Balanced = al_dup(&_t, sizeof(struct Balanced)) }; })
#define al_some_t_Inline(x) __extension__ ({ struct Inline _t = (x); (al_opt){ .tag = 1, .data.t_Inline = al_dup(&_t, sizeof(struct Inline)) }; })
#define al_some_t_LineClass(x) __extension__ ({ struct LineClass _t = (x); (al_opt){ .tag = 1, .data.t_LineClass = al_dup(&_t, sizeof(struct LineClass)) }; })
#define al_some_t_MarkerKind(x) __extension__ ({ struct MarkerKind _t = (x); (al_opt){ .tag = 1, .data.t_MarkerKind = al_dup(&_t, sizeof(struct MarkerKind)) }; })
#define al_some_t_MarkerAtom(x) __extension__ ({ struct MarkerAtom _t = (x); (al_opt){ .tag = 1, .data.t_MarkerAtom = al_dup(&_t, sizeof(struct MarkerAtom)) }; })
#define al_some_t_LwAtom(x) __extension__ ({ struct LwAtom _t = (x); (al_opt){ .tag = 1, .data.t_LwAtom = al_dup(&_t, sizeof(struct LwAtom)) }; })
#define al_some_t_AtomOut(x) __extension__ ({ struct AtomOut _t = (x); (al_opt){ .tag = 1, .data.t_AtomOut = al_dup(&_t, sizeof(struct AtomOut)) }; })
#define al_some_t_ScanOut(x) __extension__ ({ struct ScanOut _t = (x); (al_opt){ .tag = 1, .data.t_ScanOut = al_dup(&_t, sizeof(struct ScanOut)) }; })
#define al_some_t_CloseSpan(x) __extension__ ({ struct CloseSpan _t = (x); (al_opt){ .tag = 1, .data.t_CloseSpan = al_dup(&_t, sizeof(struct CloseSpan)) }; })
#define al_some_t_CloseHit(x) __extension__ ({ struct CloseHit _t = (x); (al_opt){ .tag = 1, .data.t_CloseHit = al_dup(&_t, sizeof(struct CloseHit)) }; })
#define al_some_t_Frame(x) __extension__ ({ struct Frame _t = (x); (al_opt){ .tag = 1, .data.t_Frame = al_dup(&_t, sizeof(struct Frame)) }; })
#define al_some_t_Block(x) __extension__ ({ struct Block _t = (x); (al_opt){ .tag = 1, .data.t_Block = al_dup(&_t, sizeof(struct Block)) }; })
#define al_some_t_LexedBlock(x) __extension__ ({ struct LexedBlock _t = (x); (al_opt){ .tag = 1, .data.t_LexedBlock = al_dup(&_t, sizeof(struct LexedBlock)) }; })
#define al_some_t_LexOutput(x) __extension__ ({ struct LexOutput _t = (x); (al_opt){ .tag = 1, .data.t_LexOutput = al_dup(&_t, sizeof(struct LexOutput)) }; })
#define al_some_t_CmdOutcome(x) __extension__ ({ struct CmdOutcome _t = (x); (al_opt){ .tag = 1, .data.t_CmdOutcome = al_dup(&_t, sizeof(struct CmdOutcome)) }; })
#define al_some_t_SourceSpan(x) __extension__ ({ struct SourceSpan _t = (x); (al_opt){ .tag = 1, .data.t_SourceSpan = al_dup(&_t, sizeof(struct SourceSpan)) }; })
#define al_some_t_TextPatch(x) __extension__ ({ struct TextPatch _t = (x); (al_opt){ .tag = 1, .data.t_TextPatch = al_dup(&_t, sizeof(struct TextPatch)) }; })
#define al_some_t_Buffer(x) __extension__ ({ struct Buffer _t = (x); (al_opt){ .tag = 1, .data.t_Buffer = al_dup(&_t, sizeof(struct Buffer)) }; })
#define al_some_t_BufWithPatch(x) __extension__ ({ struct BufWithPatch _t = (x); (al_opt){ .tag = 1, .data.t_BufWithPatch = al_dup(&_t, sizeof(struct BufWithPatch)) }; })
#define al_none() ((al_opt){ .tag = 0 })
struct Severity;
struct Diagnostic;
struct ExplicitCommand;
struct EscapeHit;
struct Arg;
struct KVPair;
struct CommandUse;
struct ParseErr;
struct HeaderParsed;
struct ValParsed;
struct Balanced;
struct Inline;
struct LineClass;
struct MarkerKind;
struct MarkerAtom;
struct LwAtom;
struct AtomOut;
struct ScanOut;
struct CloseSpan;
struct CloseHit;
struct Frame;
struct Block;
struct LexedBlock;
struct LexOutput;
struct CmdOutcome;
struct SourceSpan;
struct TextPatch;
struct Buffer;
struct BufWithPatch;
static int al_eq_Severity(struct Severity a, struct Severity b);
static int al_eq_Diagnostic(struct Diagnostic a, struct Diagnostic b);
static int al_eq_ExplicitCommand(struct ExplicitCommand a, struct ExplicitCommand b);
static int al_eq_EscapeHit(struct EscapeHit a, struct EscapeHit b);
static int al_eq_Arg(struct Arg a, struct Arg b);
static int al_eq_KVPair(struct KVPair a, struct KVPair b);
static int al_eq_CommandUse(struct CommandUse a, struct CommandUse b);
static int al_eq_ParseErr(struct ParseErr a, struct ParseErr b);
static int al_eq_HeaderParsed(struct HeaderParsed a, struct HeaderParsed b);
static int al_eq_ValParsed(struct ValParsed a, struct ValParsed b);
static int al_eq_Balanced(struct Balanced a, struct Balanced b);
static int al_eq_Inline(struct Inline a, struct Inline b);
static int al_eq_LineClass(struct LineClass a, struct LineClass b);
static int al_eq_MarkerKind(struct MarkerKind a, struct MarkerKind b);
static int al_eq_MarkerAtom(struct MarkerAtom a, struct MarkerAtom b);
static int al_eq_LwAtom(struct LwAtom a, struct LwAtom b);
static int al_eq_AtomOut(struct AtomOut a, struct AtomOut b);
static int al_eq_ScanOut(struct ScanOut a, struct ScanOut b);
static int al_eq_CloseSpan(struct CloseSpan a, struct CloseSpan b);
static int al_eq_CloseHit(struct CloseHit a, struct CloseHit b);
static int al_eq_Frame(struct Frame a, struct Frame b);
static int al_eq_Block(struct Block a, struct Block b);
static int al_eq_LexedBlock(struct LexedBlock a, struct LexedBlock b);
static int al_eq_LexOutput(struct LexOutput a, struct LexOutput b);
static int al_eq_CmdOutcome(struct CmdOutcome a, struct CmdOutcome b);
static int al_eq_SourceSpan(struct SourceSpan a, struct SourceSpan b);
static int al_eq_TextPatch(struct TextPatch a, struct TextPatch b);
static int al_eq_Buffer(struct Buffer a, struct Buffer b);
static int al_eq_BufWithPatch(struct BufWithPatch a, struct BufWithPatch b);
enum Severity_tag {
  SevError,
  SevWarning
};
struct Severity {
  enum Severity_tag tag;
  union {

  } data;
};
static struct Severity Severity_SevError(void) {
  struct Severity r = { .tag = SevError };
  return r;
}
static struct Severity Severity_SevWarning(void) {
  struct Severity r = { .tag = SevWarning };
  return r;
}
static int al_eq_Severity(struct Severity a, struct Severity b) {
  if (a.tag != b.tag) { return 0; }
  return 1;
}
struct Diagnostic {
  struct Severity severity;
  char* message;
  al_opt hint;
  int line;
};
static int al_eq_Diagnostic(struct Diagnostic a, struct Diagnostic b) {
  if (!(al_eq_Severity(a.severity, b.severity) && (strcmp(a.message, b.message) == 0) && ((a.hint).tag == (b.hint).tag) && (a.line == b.line))) { return 0; }
  return 1;
}
enum ExplicitCommand_tag {
  Image,
  Figure,
  Table,
  Cell,
  Font,
  Size,
  Color,
  U,
  Link,
  Ref,
  Label,
  Footnote,
  Toc,
  Comment,
  Page,
  Margin,
  Theme,
  Numbering,
  LineSpacing,
  FirstLine,
  Bold,
  Math,
  Code
};
struct ExplicitCommand {
  enum ExplicitCommand_tag tag;
  union {

  } data;
};
static struct ExplicitCommand ExplicitCommand_Image(void) {
  struct ExplicitCommand r = { .tag = Image };
  return r;
}
static struct ExplicitCommand ExplicitCommand_Figure(void) {
  struct ExplicitCommand r = { .tag = Figure };
  return r;
}
static struct ExplicitCommand ExplicitCommand_Table(void) {
  struct ExplicitCommand r = { .tag = Table };
  return r;
}
static struct ExplicitCommand ExplicitCommand_Cell(void) {
  struct ExplicitCommand r = { .tag = Cell };
  return r;
}
static struct ExplicitCommand ExplicitCommand_Font(void) {
  struct ExplicitCommand r = { .tag = Font };
  return r;
}
static struct ExplicitCommand ExplicitCommand_Size(void) {
  struct ExplicitCommand r = { .tag = Size };
  return r;
}
static struct ExplicitCommand ExplicitCommand_Color(void) {
  struct ExplicitCommand r = { .tag = Color };
  return r;
}
static struct ExplicitCommand ExplicitCommand_U(void) {
  struct ExplicitCommand r = { .tag = U };
  return r;
}
static struct ExplicitCommand ExplicitCommand_Link(void) {
  struct ExplicitCommand r = { .tag = Link };
  return r;
}
static struct ExplicitCommand ExplicitCommand_Ref(void) {
  struct ExplicitCommand r = { .tag = Ref };
  return r;
}
static struct ExplicitCommand ExplicitCommand_Label(void) {
  struct ExplicitCommand r = { .tag = Label };
  return r;
}
static struct ExplicitCommand ExplicitCommand_Footnote(void) {
  struct ExplicitCommand r = { .tag = Footnote };
  return r;
}
static struct ExplicitCommand ExplicitCommand_Toc(void) {
  struct ExplicitCommand r = { .tag = Toc };
  return r;
}
static struct ExplicitCommand ExplicitCommand_Comment(void) {
  struct ExplicitCommand r = { .tag = Comment };
  return r;
}
static struct ExplicitCommand ExplicitCommand_Page(void) {
  struct ExplicitCommand r = { .tag = Page };
  return r;
}
static struct ExplicitCommand ExplicitCommand_Margin(void) {
  struct ExplicitCommand r = { .tag = Margin };
  return r;
}
static struct ExplicitCommand ExplicitCommand_Theme(void) {
  struct ExplicitCommand r = { .tag = Theme };
  return r;
}
static struct ExplicitCommand ExplicitCommand_Numbering(void) {
  struct ExplicitCommand r = { .tag = Numbering };
  return r;
}
static struct ExplicitCommand ExplicitCommand_LineSpacing(void) {
  struct ExplicitCommand r = { .tag = LineSpacing };
  return r;
}
static struct ExplicitCommand ExplicitCommand_FirstLine(void) {
  struct ExplicitCommand r = { .tag = FirstLine };
  return r;
}
static struct ExplicitCommand ExplicitCommand_Bold(void) {
  struct ExplicitCommand r = { .tag = Bold };
  return r;
}
static struct ExplicitCommand ExplicitCommand_Math(void) {
  struct ExplicitCommand r = { .tag = Math };
  return r;
}
static struct ExplicitCommand ExplicitCommand_Code(void) {
  struct ExplicitCommand r = { .tag = Code };
  return r;
}
static int al_eq_ExplicitCommand(struct ExplicitCommand a, struct ExplicitCommand b) {
  if (a.tag != b.tag) { return 0; }
  return 1;
}
struct EscapeHit {
  int consumed;
  char* text;
};
static int al_eq_EscapeHit(struct EscapeHit a, struct EscapeHit b) {
  if (!((a.consumed == b.consumed) && (strcmp(a.text, b.text) == 0))) { return 0; }
  return 1;
}
enum Arg_tag {
  Str,
  Atom
};
struct Arg {
  enum Arg_tag tag;
  union {
    struct { char* _0; } Str;
    struct { char* _0; } Atom;
  } data;
};
static struct Arg Arg_Str(char* _0) {
  struct Arg r = { .tag = Str, .data.Str = { _0 } };
  return r;
}
static struct Arg Arg_Atom(char* _0) {
  struct Arg r = { .tag = Atom, .data.Atom = { _0 } };
  return r;
}
static int al_eq_Arg(struct Arg a, struct Arg b) {
  if (a.tag != b.tag) { return 0; }
  switch (a.tag) {
    case Str: { return (strcmp(a.data.Str._0, b.data.Str._0) == 0); }
    case Atom: { return (strcmp(a.data.Atom._0, b.data.Atom._0) == 0); }
    default: { break; }
  }
  return 1;
}
struct KVPair {
  char* key;
  struct Arg val;
};
static int al_eq_KVPair(struct KVPair a, struct KVPair b) {
  if (!((strcmp(a.key, b.key) == 0) && al_eq_Arg(a.val, b.val))) { return 0; }
  return 1;
}
struct CommandUse {
  struct ExplicitCommand cmd;
  char* name_raw;
  al_vec args;
  al_vec attrs;
  al_opt content;
};
static int al_eq_CommandUse(struct CommandUse a, struct CommandUse b) {
  if (!(al_eq_ExplicitCommand(a.cmd, b.cmd) && (strcmp(a.name_raw, b.name_raw) == 0) && al_vec_eq(a.args, b.args) && al_vec_eq(a.attrs, b.attrs) && ((a.content).tag == (b.content).tag))) { return 0; }
  return 1;
}
struct ParseErr {
  char* msg;
  al_opt hint;
};
static int al_eq_ParseErr(struct ParseErr a, struct ParseErr b) {
  if (!((strcmp(a.msg, b.msg) == 0) && ((a.hint).tag == (b.hint).tag))) { return 0; }
  return 1;
}
struct HeaderParsed {
  struct CommandUse cu;
  int consumed;
};
static int al_eq_HeaderParsed(struct HeaderParsed a, struct HeaderParsed b) {
  if (!(al_eq_CommandUse(a.cu, b.cu) && (a.consumed == b.consumed))) { return 0; }
  return 1;
}
struct ValParsed {
  struct Arg arg;
  int consumed;
};
static int al_eq_ValParsed(struct ValParsed a, struct ValParsed b) {
  if (!(al_eq_Arg(a.arg, b.arg) && (a.consumed == b.consumed))) { return 0; }
  return 1;
}
struct Balanced {
  char* text;
  int close_idx;
};
static int al_eq_Balanced(struct Balanced a, struct Balanced b) {
  if (!((strcmp(a.text, b.text) == 0) && (a.close_idx == b.close_idx))) { return 0; }
  return 1;
}
enum Inline_tag {
  AwTxt,
  IBold,
  Italic,
  Strike,
  Scoped,
  CodeSpan,
  Command,
  RawInline
};
struct Inline {
  enum Inline_tag tag;
  union {
    struct { char* _0; } AwTxt;
    struct { al_vec _0; } IBold;
    struct { al_vec _0; } Italic;
    struct { al_vec _0; } Strike;
    struct { struct ExplicitCommand _0; al_vec _1; } Scoped;
    struct { char* _0; } CodeSpan;
    struct { struct CommandUse _0; } Command;
    struct { struct ExplicitCommand _0; al_opt _1; char* _2; } RawInline;
  } data;
};
static struct Inline Inline_AwTxt(char* _0) {
  struct Inline r = { .tag = AwTxt, .data.AwTxt = { _0 } };
  return r;
}
static struct Inline Inline_IBold(al_vec _0) {
  struct Inline r = { .tag = IBold, .data.IBold = { _0 } };
  return r;
}
static struct Inline Inline_Italic(al_vec _0) {
  struct Inline r = { .tag = Italic, .data.Italic = { _0 } };
  return r;
}
static struct Inline Inline_Strike(al_vec _0) {
  struct Inline r = { .tag = Strike, .data.Strike = { _0 } };
  return r;
}
static struct Inline Inline_Scoped(struct ExplicitCommand _0, al_vec _1) {
  struct Inline r = { .tag = Scoped, .data.Scoped = { _0, _1 } };
  return r;
}
static struct Inline Inline_CodeSpan(char* _0) {
  struct Inline r = { .tag = CodeSpan, .data.CodeSpan = { _0 } };
  return r;
}
static struct Inline Inline_Command(struct CommandUse _0) {
  struct Inline r = { .tag = Command, .data.Command = { _0 } };
  return r;
}
static struct Inline Inline_RawInline(struct ExplicitCommand _0, al_opt _1, char* _2) {
  struct Inline r = { .tag = RawInline, .data.RawInline = { _0, _1, _2 } };
  return r;
}
static int al_eq_Inline(struct Inline a, struct Inline b) {
  if (a.tag != b.tag) { return 0; }
  switch (a.tag) {
    case AwTxt: { return (strcmp(a.data.AwTxt._0, b.data.AwTxt._0) == 0); }
    case IBold: { return al_vec_eq(a.data.IBold._0, b.data.IBold._0); }
    case Italic: { return al_vec_eq(a.data.Italic._0, b.data.Italic._0); }
    case Strike: { return al_vec_eq(a.data.Strike._0, b.data.Strike._0); }
    case Scoped: { return al_eq_ExplicitCommand(a.data.Scoped._0, b.data.Scoped._0) && al_vec_eq(a.data.Scoped._1, b.data.Scoped._1); }
    case CodeSpan: { return (strcmp(a.data.CodeSpan._0, b.data.CodeSpan._0) == 0); }
    case Command: { return al_eq_CommandUse(a.data.Command._0, b.data.Command._0); }
    case RawInline: { return al_eq_ExplicitCommand(a.data.RawInline._0, b.data.RawInline._0) && ((a.data.RawInline._1).tag == (b.data.RawInline._1).tag) && (strcmp(a.data.RawInline._2, b.data.RawInline._2) == 0); }
    default: { break; }
  }
  return 1;
}
enum LineClass_tag {
  LcBlank,
  LcDivider,
  LcHeading,
  LcQuote,
  LcListItem,
  LcCommand,
  LcText
};
struct LineClass {
  enum LineClass_tag tag;
  union {
    struct { int _0; } LcHeading;
    struct { int _0; } LcQuote;
    struct { int _0; } LcListItem;
  } data;
};
static struct LineClass LineClass_LcBlank(void) {
  struct LineClass r = { .tag = LcBlank };
  return r;
}
static struct LineClass LineClass_LcDivider(void) {
  struct LineClass r = { .tag = LcDivider };
  return r;
}
static struct LineClass LineClass_LcHeading(int _0) {
  struct LineClass r = { .tag = LcHeading, .data.LcHeading = { _0 } };
  return r;
}
static struct LineClass LineClass_LcQuote(int _0) {
  struct LineClass r = { .tag = LcQuote, .data.LcQuote = { _0 } };
  return r;
}
static struct LineClass LineClass_LcListItem(int _0) {
  struct LineClass r = { .tag = LcListItem, .data.LcListItem = { _0 } };
  return r;
}
static struct LineClass LineClass_LcCommand(void) {
  struct LineClass r = { .tag = LcCommand };
  return r;
}
static struct LineClass LineClass_LcText(void) {
  struct LineClass r = { .tag = LcText };
  return r;
}
static int al_eq_LineClass(struct LineClass a, struct LineClass b) {
  if (a.tag != b.tag) { return 0; }
  switch (a.tag) {
    case LcHeading: { return (a.data.LcHeading._0 == b.data.LcHeading._0); }
    case LcQuote: { return (a.data.LcQuote._0 == b.data.LcQuote._0); }
    case LcListItem: { return (a.data.LcListItem._0 == b.data.LcListItem._0); }
    default: { break; }
  }
  return 1;
}
enum MarkerKind_tag {
  MkBold,
  MkItalic,
  MkStrike,
  MkExplicit
};
struct MarkerKind {
  enum MarkerKind_tag tag;
  union {
    struct { struct ExplicitCommand _0; } MkExplicit;
  } data;
};
static struct MarkerKind MarkerKind_MkBold(void) {
  struct MarkerKind r = { .tag = MkBold };
  return r;
}
static struct MarkerKind MarkerKind_MkItalic(void) {
  struct MarkerKind r = { .tag = MkItalic };
  return r;
}
static struct MarkerKind MarkerKind_MkStrike(void) {
  struct MarkerKind r = { .tag = MkStrike };
  return r;
}
static struct MarkerKind MarkerKind_MkExplicit(struct ExplicitCommand _0) {
  struct MarkerKind r = { .tag = MkExplicit, .data.MkExplicit = { _0 } };
  return r;
}
static int al_eq_MarkerKind(struct MarkerKind a, struct MarkerKind b) {
  if (a.tag != b.tag) { return 0; }
  switch (a.tag) {
    case MkExplicit: { return al_eq_ExplicitCommand(a.data.MkExplicit._0, b.data.MkExplicit._0); }
    default: { break; }
  }
  return 1;
}
struct MarkerAtom {
  struct MarkerKind kind;
  char* literal;
  int can_open;
  int can_close;
};
static int al_eq_MarkerAtom(struct MarkerAtom a, struct MarkerAtom b) {
  if (!(al_eq_MarkerKind(a.kind, b.kind) && (strcmp(a.literal, b.literal) == 0) && (a.can_open == b.can_open) && (a.can_close == b.can_close))) { return 0; }
  return 1;
}
enum LwAtom_tag {
  AtText,
  AtMarker,
  AtCode,
  AtCmd,
  AtClose,
  AtRawSeg
};
struct LwAtom {
  enum LwAtom_tag tag;
  union {
    struct { char* _0; } AtText;
    struct { struct MarkerAtom _0; } AtMarker;
    struct { char* _0; } AtCode;
    struct { struct CommandUse _0; char* _1; int _2; } AtCmd;
    struct { struct ExplicitCommand _0; char* _1; } AtClose;
    struct { struct ExplicitCommand _0; al_opt _1; char* _2; char* _3; } AtRawSeg;
  } data;
};
static struct LwAtom LwAtom_AtText(char* _0) {
  struct LwAtom r = { .tag = AtText, .data.AtText = { _0 } };
  return r;
}
static struct LwAtom LwAtom_AtMarker(struct MarkerAtom _0) {
  struct LwAtom r = { .tag = AtMarker, .data.AtMarker = { _0 } };
  return r;
}
static struct LwAtom LwAtom_AtCode(char* _0) {
  struct LwAtom r = { .tag = AtCode, .data.AtCode = { _0 } };
  return r;
}
static struct LwAtom LwAtom_AtCmd(struct CommandUse _0, char* _1, int _2) {
  struct LwAtom r = { .tag = AtCmd, .data.AtCmd = { _0, _1, _2 } };
  return r;
}
static struct LwAtom LwAtom_AtClose(struct ExplicitCommand _0, char* _1) {
  struct LwAtom r = { .tag = AtClose, .data.AtClose = { _0, _1 } };
  return r;
}
static struct LwAtom LwAtom_AtRawSeg(struct ExplicitCommand _0, al_opt _1, char* _2, char* _3) {
  struct LwAtom r = { .tag = AtRawSeg, .data.AtRawSeg = { _0, _1, _2, _3 } };
  return r;
}
static int al_eq_LwAtom(struct LwAtom a, struct LwAtom b) {
  if (a.tag != b.tag) { return 0; }
  switch (a.tag) {
    case AtText: { return (strcmp(a.data.AtText._0, b.data.AtText._0) == 0); }
    case AtMarker: { return al_eq_MarkerAtom(a.data.AtMarker._0, b.data.AtMarker._0); }
    case AtCode: { return (strcmp(a.data.AtCode._0, b.data.AtCode._0) == 0); }
    case AtCmd: { return al_eq_CommandUse(a.data.AtCmd._0, b.data.AtCmd._0) && (strcmp(a.data.AtCmd._1, b.data.AtCmd._1) == 0) && (a.data.AtCmd._2 == b.data.AtCmd._2); }
    case AtClose: { return al_eq_ExplicitCommand(a.data.AtClose._0, b.data.AtClose._0) && (strcmp(a.data.AtClose._1, b.data.AtClose._1) == 0); }
    case AtRawSeg: { return al_eq_ExplicitCommand(a.data.AtRawSeg._0, b.data.AtRawSeg._0) && ((a.data.AtRawSeg._1).tag == (b.data.AtRawSeg._1).tag) && (strcmp(a.data.AtRawSeg._2, b.data.AtRawSeg._2) == 0) && (strcmp(a.data.AtRawSeg._3, b.data.AtRawSeg._3) == 0); }
    default: { break; }
  }
  return 1;
}
struct AtomOut {
  al_vec atoms;
  al_vec diags;
};
static int al_eq_AtomOut(struct AtomOut a, struct AtomOut b) {
  if (!(al_vec_eq(a.atoms, b.atoms) && al_vec_eq(a.diags, b.diags))) { return 0; }
  return 1;
}
struct ScanOut {
  al_vec inline_a;
  al_vec diags;
};
static int al_eq_ScanOut(struct ScanOut a, struct ScanOut b) {
  if (!(al_vec_eq(a.inline_a, b.inline_a) && al_vec_eq(a.diags, b.diags))) { return 0; }
  return 1;
}
struct CloseSpan {
  int start;
  int end;
};
static int al_eq_CloseSpan(struct CloseSpan a, struct CloseSpan b) {
  if (!((a.start == b.start) && (a.end == b.end))) { return 0; }
  return 1;
}
struct CloseHit {
  struct ExplicitCommand cmd;
  int end;
};
static int al_eq_CloseHit(struct CloseHit a, struct CloseHit b) {
  if (!(al_eq_ExplicitCommand(a.cmd, b.cmd) && (a.end == b.end))) { return 0; }
  return 1;
}
struct Frame {
  al_opt kind;
  al_vec children;
};
static int al_eq_Frame(struct Frame a, struct Frame b) {
  if (!(((a.kind).tag == (b.kind).tag) && al_vec_eq(a.children, b.children))) { return 0; }
  return 1;
}
enum Block_tag {
  Blank,
  Heading,
  Paragraph,
  ListItem,
  Quote,
  Divider,
  Object,
  RawOpen,
  RawLine,
  RawClose,
  TableOpen,
  TableRow,
  TableClose
};
struct Block {
  enum Block_tag tag;
  union {
    struct { int _0; al_vec _1; } Heading;
    struct { al_vec _0; } Paragraph;
    struct { int _0; al_vec _1; } ListItem;
    struct { int _0; al_vec _1; } Quote;
    struct { struct CommandUse _0; } Object;
    struct { struct ExplicitCommand _0; al_opt _1; } RawOpen;
    struct { char* _0; } RawLine;
    struct { struct ExplicitCommand _0; } RawClose;
    struct { struct CommandUse _0; } TableOpen;
    struct { al_vec _0; int _1; } TableRow;
  } data;
};
static struct Block Block_Blank(void) {
  struct Block r = { .tag = Blank };
  return r;
}
static struct Block Block_Heading(int _0, al_vec _1) {
  struct Block r = { .tag = Heading, .data.Heading = { _0, _1 } };
  return r;
}
static struct Block Block_Paragraph(al_vec _0) {
  struct Block r = { .tag = Paragraph, .data.Paragraph = { _0 } };
  return r;
}
static struct Block Block_ListItem(int _0, al_vec _1) {
  struct Block r = { .tag = ListItem, .data.ListItem = { _0, _1 } };
  return r;
}
static struct Block Block_Quote(int _0, al_vec _1) {
  struct Block r = { .tag = Quote, .data.Quote = { _0, _1 } };
  return r;
}
static struct Block Block_Divider(void) {
  struct Block r = { .tag = Divider };
  return r;
}
static struct Block Block_Object(struct CommandUse _0) {
  struct Block r = { .tag = Object, .data.Object = { _0 } };
  return r;
}
static struct Block Block_RawOpen(struct ExplicitCommand _0, al_opt _1) {
  struct Block r = { .tag = RawOpen, .data.RawOpen = { _0, _1 } };
  return r;
}
static struct Block Block_RawLine(char* _0) {
  struct Block r = { .tag = RawLine, .data.RawLine = { _0 } };
  return r;
}
static struct Block Block_RawClose(struct ExplicitCommand _0) {
  struct Block r = { .tag = RawClose, .data.RawClose = { _0 } };
  return r;
}
static struct Block Block_TableOpen(struct CommandUse _0) {
  struct Block r = { .tag = TableOpen, .data.TableOpen = { _0 } };
  return r;
}
static struct Block Block_TableRow(al_vec _0, int _1) {
  struct Block r = { .tag = TableRow, .data.TableRow = { _0, _1 } };
  return r;
}
static struct Block Block_TableClose(void) {
  struct Block r = { .tag = TableClose };
  return r;
}
static int al_eq_Block(struct Block a, struct Block b) {
  if (a.tag != b.tag) { return 0; }
  switch (a.tag) {
    case Heading: { return (a.data.Heading._0 == b.data.Heading._0) && al_vec_eq(a.data.Heading._1, b.data.Heading._1); }
    case Paragraph: { return al_vec_eq(a.data.Paragraph._0, b.data.Paragraph._0); }
    case ListItem: { return (a.data.ListItem._0 == b.data.ListItem._0) && al_vec_eq(a.data.ListItem._1, b.data.ListItem._1); }
    case Quote: { return (a.data.Quote._0 == b.data.Quote._0) && al_vec_eq(a.data.Quote._1, b.data.Quote._1); }
    case Object: { return al_eq_CommandUse(a.data.Object._0, b.data.Object._0); }
    case RawOpen: { return al_eq_ExplicitCommand(a.data.RawOpen._0, b.data.RawOpen._0) && ((a.data.RawOpen._1).tag == (b.data.RawOpen._1).tag); }
    case RawLine: { return (strcmp(a.data.RawLine._0, b.data.RawLine._0) == 0); }
    case RawClose: { return al_eq_ExplicitCommand(a.data.RawClose._0, b.data.RawClose._0); }
    case TableOpen: { return al_eq_CommandUse(a.data.TableOpen._0, b.data.TableOpen._0); }
    case TableRow: { return al_vec_eq(a.data.TableRow._0, b.data.TableRow._0) && (a.data.TableRow._1 == b.data.TableRow._1); }
    default: { break; }
  }
  return 1;
}
struct LexedBlock {
  int line;
  struct Block block;
};
static int al_eq_LexedBlock(struct LexedBlock a, struct LexedBlock b) {
  if (!((a.line == b.line) && al_eq_Block(a.block, b.block))) { return 0; }
  return 1;
}
struct LexOutput {
  al_vec blocks;
  al_vec diags;
};
static int al_eq_LexOutput(struct LexOutput a, struct LexOutput b) {
  if (!(al_vec_eq(a.blocks, b.blocks) && al_vec_eq(a.diags, b.diags))) { return 0; }
  return 1;
}
struct CmdOutcome {
  al_vec blocks;
  al_vec diags;
  al_opt raw;
  int table;
};
static int al_eq_CmdOutcome(struct CmdOutcome a, struct CmdOutcome b) {
  if (!(al_vec_eq(a.blocks, b.blocks) && al_vec_eq(a.diags, b.diags) && ((a.raw).tag == (b.raw).tag) && (a.table == b.table))) { return 0; }
  return 1;
}
struct SourceSpan {
  int start;
  int end;
};
static int al_eq_SourceSpan(struct SourceSpan a, struct SourceSpan b) {
  if (!((a.start == b.start) && (a.end == b.end))) { return 0; }
  return 1;
}
struct TextPatch {
  int start;
  char* old;
  char* new;
};
static int al_eq_TextPatch(struct TextPatch a, struct TextPatch b) {
  if (!((a.start == b.start) && (strcmp(a.old, b.old) == 0) && (strcmp(a.new, b.new) == 0))) { return 0; }
  return 1;
}
struct Buffer {
  char* content;
};
static int al_eq_Buffer(struct Buffer a, struct Buffer b) {
  if (!((strcmp(a.content, b.content) == 0))) { return 0; }
  return 1;
}
struct BufWithPatch {
  struct Buffer buf;
  struct TextPatch patch;
};
static int al_eq_BufWithPatch(struct BufWithPatch a, struct BufWithPatch b) {
  if (!(al_eq_Buffer(a.buf, b.buf) && al_eq_TextPatch(a.patch, b.patch))) { return 0; }
  return 1;
}
struct Diagnostic diag_error(char* message, int line);
struct Diagnostic diag_warning(char* message, int line);
struct Diagnostic diag_hint(struct Diagnostic d, char* hint);
al_opt lookup(char* name);
int is_raw_block(struct ExplicitCommand cmd);
int is_doc_level(struct ExplicitCommand cmd);
int is_scoped(struct ExplicitCommand cmd);
int close_matches(struct ExplicitCommand cmd, char* close_name);
al_opt positional_arity(struct ExplicitCommand cmd);
al_opt fw_normalize_in_command(char* c);
al_opt fw_normalize_comma_in_row(char* c);
int fw_is_ideo_space(char* c);
int starts_with_at(char* s, int i, char* pat);
al_opt escape_scan(char* s, int i);
char* arg_plain(struct Arg a);
int ex_is_name_char(char* c);
int ex_is_atom_char(char* c);
struct ParseErr space_err();
al_opt ex_scan_balanced(char* s, int start);
al_opt read_value(char* s, int start);
al_opt parse_header(char* s, int at);
char* inline_plain(al_vec items);
struct LineClass classify(char* line);
int ordered_marker_len(char* line);
int lw_is_word_char(char* c);
struct ScanOut lw_scan_inline(char* chars, int line, al_vec diags);
al_opt lw_find_char(char* s, int from, char* target);
al_opt lw_code_lang(struct CommandUse u);
al_opt lw_find_inline_close(char* s, int from, struct ExplicitCommand cmd);
al_opt lw_scan_literal_object(char* s, int start);
al_opt lw_read_close_tag(char* s, int start);
struct AtomOut lw_atomize(char* chars, int line, al_vec diags);
al_vec lw_vec_set(al_vec v, int i, struct Frame f);
al_vec lw_vec_drop_last(al_vec v);
char* lw_atom_raw(struct LwAtom a);
al_vec lw_literalize(al_vec atoms);
al_vec lw_merge_texts(al_vec items);
al_vec lw_vec_set_inline(al_vec v, int i, struct Inline item);
struct Inline mk_text(char* s);
struct ScanOut lw_resolve(al_vec atoms, al_vec diags, int line);
int lw_frames_contain(al_vec frames, struct MarkerKind kind);
char* lex_strip_cr(char* line);
al_opt lex_close_tag_name(char* line);
int lex_is_delimiter_cell(char* c);
struct LexOutput lex(char* src);
struct CmdOutcome lex_command_line(char* chars, int lineno, al_vec diags, al_opt raw, int table);
al_opt span_new(int start, int end);
al_opt span_after_edit(struct SourceSpan s, int e_start, int e_end, int new_len);
int label_char_ok(char* c);
al_opt label_parse(char* s);
struct TextPatch patch_invert(struct TextPatch p);
struct Buffer buffer_new(char* s);
al_opt buffer_insert(struct Buffer b, int start, char* s);
al_opt buffer_delete(struct Buffer b, int start, int end);
al_opt buffer_apply(struct Buffer b, struct TextPatch p);
int main(int _argc, char** _argv);
struct Diagnostic diag_error(char* message, int line) {
  {
  return ((struct Diagnostic){.severity = ((struct Severity){ .tag = SevError }), .message = message, .hint = al_none(), .line = line});
  }
  return ((struct Diagnostic){ 0 });
}
struct Diagnostic diag_warning(char* message, int line) {
  {
  return ((struct Diagnostic){.severity = ((struct Severity){ .tag = SevWarning }), .message = message, .hint = al_none(), .line = line});
  }
  return ((struct Diagnostic){ 0 });
}
struct Diagnostic diag_hint(struct Diagnostic d, char* hint) {
  {
  return ((struct Diagnostic){.severity = d.severity, .message = d.message, .hint = al_some_s(hint), .line = d.line});
  }
  return ((struct Diagnostic){ 0 });
}
al_opt lookup(char* name) {
  {
  if (((strcmp(name, "image") == 0) || (strcmp(name, "图片") == 0))) {
    return al_some_t_ExplicitCommand(ExplicitCommand_Image());
  }
  if ((strcmp(name, "figure") == 0)) {
    return al_some_t_ExplicitCommand(ExplicitCommand_Figure());
  }
  if ((strcmp(name, "table") == 0)) {
    return al_some_t_ExplicitCommand(ExplicitCommand_Table());
  }
  if ((strcmp(name, "cell") == 0)) {
    return al_some_t_ExplicitCommand(ExplicitCommand_Cell());
  }
  if (((strcmp(name, "font") == 0) || (strcmp(name, "字体") == 0))) {
    return al_some_t_ExplicitCommand(ExplicitCommand_Font());
  }
  if (((strcmp(name, "size") == 0) || (strcmp(name, "大小") == 0))) {
    return al_some_t_ExplicitCommand(ExplicitCommand_Size());
  }
  if ((strcmp(name, "color") == 0)) {
    return al_some_t_ExplicitCommand(ExplicitCommand_Color());
  }
  if (((strcmp(name, "u") == 0) || (strcmp(name, "underline") == 0))) {
    return al_some_t_ExplicitCommand(ExplicitCommand_U());
  }
  if ((strcmp(name, "link") == 0)) {
    return al_some_t_ExplicitCommand(ExplicitCommand_Link());
  }
  if ((strcmp(name, "ref") == 0)) {
    return al_some_t_ExplicitCommand(ExplicitCommand_Ref());
  }
  if (((strcmp(name, "label") == 0) || (strcmp(name, "标签") == 0))) {
    return al_some_t_ExplicitCommand(ExplicitCommand_Label());
  }
  if ((strcmp(name, "footnote") == 0)) {
    return al_some_t_ExplicitCommand(ExplicitCommand_Footnote());
  }
  if ((strcmp(name, "toc") == 0)) {
    return al_some_t_ExplicitCommand(ExplicitCommand_Toc());
  }
  if ((strcmp(name, "comment") == 0)) {
    return al_some_t_ExplicitCommand(ExplicitCommand_Comment());
  }
  if ((strcmp(name, "page") == 0)) {
    return al_some_t_ExplicitCommand(ExplicitCommand_Page());
  }
  if ((strcmp(name, "margin") == 0)) {
    return al_some_t_ExplicitCommand(ExplicitCommand_Margin());
  }
  if ((strcmp(name, "theme") == 0)) {
    return al_some_t_ExplicitCommand(ExplicitCommand_Theme());
  }
  if ((strcmp(name, "numbering") == 0)) {
    return al_some_t_ExplicitCommand(ExplicitCommand_Numbering());
  }
  if ((strcmp(name, "line-spacing") == 0)) {
    return al_some_t_ExplicitCommand(ExplicitCommand_LineSpacing());
  }
  if ((strcmp(name, "first-line") == 0)) {
    return al_some_t_ExplicitCommand(ExplicitCommand_FirstLine());
  }
  if (((strcmp(name, "bold") == 0) || (strcmp(name, "b") == 0))) {
    return al_some_t_ExplicitCommand(ExplicitCommand_Bold());
  }
  if (((strcmp(name, "math") == 0) || (strcmp(name, "m") == 0))) {
    return al_some_t_ExplicitCommand(ExplicitCommand_Math());
  }
  if (((strcmp(name, "code") == 0) || (strcmp(name, "c") == 0))) {
    return al_some_t_ExplicitCommand(ExplicitCommand_Code());
  }
  return al_none();
  }
  return al_none();
}
int is_raw_block(struct ExplicitCommand cmd) {
  {
  return (((((cmd).tag) == ((ExplicitCommand_Math()).tag)) || (((cmd).tag) == ((ExplicitCommand_Code()).tag))) || (((cmd).tag) == ((ExplicitCommand_Comment()).tag)));
  }
  return 0;
}
int is_doc_level(struct ExplicitCommand cmd) {
  {
  return ((((((((((cmd).tag) == ((ExplicitCommand_Page()).tag)) || (((cmd).tag) == ((ExplicitCommand_Margin()).tag))) || (((cmd).tag) == ((ExplicitCommand_Font()).tag))) || (((cmd).tag) == ((ExplicitCommand_Size()).tag))) || (((cmd).tag) == ((ExplicitCommand_LineSpacing()).tag))) || (((cmd).tag) == ((ExplicitCommand_FirstLine()).tag))) || (((cmd).tag) == ((ExplicitCommand_Theme()).tag))) || (((cmd).tag) == ((ExplicitCommand_Numbering()).tag)));
  }
  return 0;
}
int is_scoped(struct ExplicitCommand cmd) {
  {
  return (((((((cmd).tag) == ((ExplicitCommand_Bold()).tag)) || (((cmd).tag) == ((ExplicitCommand_Font()).tag))) || (((cmd).tag) == ((ExplicitCommand_Size()).tag))) || (((cmd).tag) == ((ExplicitCommand_Color()).tag))) || (((cmd).tag) == ((ExplicitCommand_U()).tag)));
  }
  return 0;
}
int close_matches(struct ExplicitCommand cmd, char* close_name) {
  {
  if ((((cmd).tag) == ((ExplicitCommand_Code()).tag))) {
    return ((strcmp(close_name, "c") == 0) || (strcmp(close_name, "code") == 0));
  }
  if ((((cmd).tag) == ((ExplicitCommand_Math()).tag))) {
    return ((strcmp(close_name, "m") == 0) || (strcmp(close_name, "math") == 0));
  }
  return __extension__ ({ al_opt _l = (lookup(close_name)); al_opt _r = (al_some_t_ExplicitCommand(cmd)); (_l.tag == _r.tag && (_l.tag == 0 || al_eq_ExplicitCommand(*_l.data.t_ExplicitCommand, *_r.data.t_ExplicitCommand))); });
  }
  return 0;
}
al_opt positional_arity(struct ExplicitCommand cmd) {
  {
  if (((((((((((cmd).tag) == ((ExplicitCommand_Font()).tag)) || (((cmd).tag) == ((ExplicitCommand_Size()).tag))) || (((cmd).tag) == ((ExplicitCommand_Color()).tag))) || (((cmd).tag) == ((ExplicitCommand_Theme()).tag))) || (((cmd).tag) == ((ExplicitCommand_Page()).tag))) || (((cmd).tag) == ((ExplicitCommand_Margin()).tag))) || (((cmd).tag) == ((ExplicitCommand_LineSpacing()).tag))) || (((cmd).tag) == ((ExplicitCommand_FirstLine()).tag)))) {
    return al_some_i(1);
  }
  if (((((((cmd).tag) == ((ExplicitCommand_Bold()).tag)) || (((cmd).tag) == ((ExplicitCommand_U()).tag))) || (((cmd).tag) == ((ExplicitCommand_Math()).tag))) || (((cmd).tag) == ((ExplicitCommand_Toc()).tag)))) {
    return al_some_i(0);
  }
  if ((((cmd).tag) == ((ExplicitCommand_Code()).tag))) {
    return al_some_i(1);
  }
  return al_none();
  }
  return al_none();
}
al_opt fw_normalize_in_command(char* c) {
  {
  if (((((strcmp(c, "“") == 0) || (strcmp(c, "”") == 0)) || (strcmp(c, "‘") == 0)) || (strcmp(c, "’") == 0))) {
    return al_some_s("\"");
  }
  if ((strcmp(c, "：") == 0)) {
    return al_some_s(":");
  }
  return al_none();
  }
  return al_none();
}
al_opt fw_normalize_comma_in_row(char* c) {
  {
  if ((strcmp(c, "，") == 0)) {
    return al_some_s(",");
  }
  return al_none();
  }
  return al_none();
}
int fw_is_ideo_space(char* c) {
  {
  return (strcmp(c, "　") == 0);
  }
  return 0;
}
int starts_with_at(char* s, int i, char* pat) {
  {
  if (((i < 0) || ((i + strlen(pat)) > strlen(s)))) {
    return 0;
  }
  return (strcmp(al_substr_ch(s, i, (i + strlen(pat))), pat) == 0);
  }
  return 0;
}
al_opt escape_scan(char* s, int i) {
  {
  if (starts_with_at(s, i, "@**")) {
    return al_some_t_EscapeHit(((struct EscapeHit){.consumed = 3, .text = "**"}));
  }
  if (starts_with_at(s, i, "@~~")) {
    return al_some_t_EscapeHit(((struct EscapeHit){.consumed = 3, .text = "~~"}));
  }
  if (starts_with_at(s, i, "@#")) {
    return al_some_t_EscapeHit(((struct EscapeHit){.consumed = 2, .text = "#"}));
  }
  if (starts_with_at(s, i, "@_")) {
    return al_some_t_EscapeHit(((struct EscapeHit){.consumed = 2, .text = "_"}));
  }
  if (starts_with_at(s, i, "@-")) {
    return al_some_t_EscapeHit(((struct EscapeHit){.consumed = 2, .text = "-"}));
  }
  if (starts_with_at(s, i, "@>")) {
    return al_some_t_EscapeHit(((struct EscapeHit){.consumed = 2, .text = ">"}));
  }
  if (starts_with_at(s, i, "@`")) {
    return al_some_t_EscapeHit(((struct EscapeHit){.consumed = 2, .text = "`"}));
  }
  int j = (i + 1); // var j
  while ((((j < strlen(s)) && (s[j] >= '0')) && (s[j] <= '9'))) {
    j += 1;
  }
  if ((((j > (i + 1)) && (j < strlen(s))) && (s[j] == '.'))) {
    return al_some_t_EscapeHit(((struct EscapeHit){.consumed = ((j - i) + 1), .text = al_substr_ch(s, (i + 1), (j + 1))}));
  }
  return al_none();
  }
  return al_none();
}
char* arg_plain(struct Arg a) {
  {
  if (a.tag == Str) {
    char* s = a.data.Str._0;
    {
      return s;
    }
  }
  else if (a.tag == Atom) {
    char* s = a.data.Atom._0;
    {
      return s;
    }
  }
  }
  return "";
}
int ex_is_name_char(char* c) {
  {
  if (fw_is_ideo_space(c)) {
    return 0;
  }
  if (((((((strcmp(c, "：") == 0) || (strcmp(c, "“") == 0)) || (strcmp(c, "”") == 0)) || (strcmp(c, "‘") == 0)) || (strcmp(c, "’") == 0)) || (strcmp(c, "，") == 0))) {
    return 0;
  }
  if (((strcmp(c, "a") >= 0) && (strcmp(c, "z") <= 0))) {
    return 1;
  }
  if (((strcmp(c, "A") >= 0) && (strcmp(c, "Z") <= 0))) {
    return 1;
  }
  if (((strcmp(c, "0") >= 0) && (strcmp(c, "9") <= 0))) {
    return 1;
  }
  if ((strcmp(c, "-") == 0)) {
    return 1;
  }
  if ((strcmp(c, "\\x7f") > 0)) {
    return 1;
  }
  return 0;
  }
  return 0;
}
int ex_is_atom_char(char* c) {
  {
  if ((((((strcmp(c, " ") == 0) || (strcmp(c, "\t") == 0)) || (strcmp(c, "]") == 0)) || (strcmp(c, ":") == 0)) || (strcmp(c, "\"") == 0))) {
    return 0;
  }
  if ((((((strcmp(c, "：") == 0) || (strcmp(c, "“") == 0)) || (strcmp(c, "”") == 0)) || (strcmp(c, "‘") == 0)) || (strcmp(c, "’") == 0))) {
    return 0;
  }
  if (fw_is_ideo_space(c)) {
    return 0;
  }
  return 1;
  }
  return 0;
}
struct ParseErr space_err() {
  {
  return ((struct ParseErr){.msg = "全角空格(U+3000)不是语法分隔符", .hint = al_some_s("未加引号的值不能包含空格,请使用双引号(规范 §6.10)")});
  }
  return ((struct ParseErr){ 0 });
}
al_opt ex_scan_balanced(char* s, int start) {
  {
  int depth = 1; // var depth
  char* out = al_strdup_lit(""); // var out
  int i = start; // var i
  while ((i < strlen(s))) {
    char c = s[i]; // let c
    if ((c == '[')) {
      depth += 1;
      out = al_strcat_own(out, "[");
    } else {
      if ((c == ']')) {
        depth -= 1;
        if ((depth == 0)) {
          return al_some_t_Balanced(((struct Balanced){.text = out, .close_idx = i}));
        }
        out = al_strcat_own(out, "]");
      } else {
        out = al_strcat_own(out, al_char_to_str(c));
      }
    }
    i += 1;
  }
  return al_none();
  }
  return al_none();
}
al_opt read_value(char* s, int start) {
  {
  char raw_c = s[start]; // let raw_c
  al_opt q_opt = fw_normalize_in_command(al_char_to_str(raw_c)); // let q_opt
  char* quote = (q_opt.tag == 1 ? ({ char* q = q_opt.data.s; q; }) : (q_opt.tag == 0 ? al_char_to_str(raw_c) : "")); // let quote
  if ((strcmp(quote, "\"") == 0)) {
    int i = (start + 1); // var i
    char* out = al_strdup_lit(""); // var out
    while ((i < strlen(s))) {
      al_opt cq_opt = fw_normalize_in_command(al_char_to_str(s[i])); // let cq_opt
      char* cq = (cq_opt.tag == 1 ? ({ char* q = cq_opt.data.s; q; }) : (cq_opt.tag == 0 ? al_char_to_str(s[i]) : "")); // let cq
      if ((strcmp(cq, "\"") == 0)) {
        return al_some_t_ValParsed(((struct ValParsed){.arg = Arg_Str(out), .consumed = ((i + 1) - start)}));
      }
      out = al_strcat_own(out, al_char_to_str(s[i]));
      i += 1;
    }
    return al_none();
  }
  int i = start; // var i
  while (((i < strlen(s)) && ex_is_atom_char(al_char_to_str(s[i])))) {
    i += 1;
  }
  if ((i == start)) {
    return al_none();
  }
  return al_some_t_ValParsed(((struct ValParsed){.arg = Arg_Atom(al_substr_ch(s, start, i)), .consumed = (i - start)}));
  }
  return al_none();
}
al_opt parse_header(char* s, int at) {
  {
  int i = (at + 2); // var i
  int name_start = i; // let name_start
  while (((i < strlen(s)) && ex_is_name_char(al_char_to_str(s[i])))) {
    i += 1;
  }
  char* name = al_substr_ch(s, name_start, i); // let name
  if ((strlen(name) == 0)) {
    return al_none();
  }
  al_opt cmd_opt = lookup(name); // let cmd_opt
  struct ExplicitCommand cmd = __extension__ ({ struct ExplicitCommand _mv = ((struct ExplicitCommand){ 0 }); if (cmd_opt.tag == 1) { struct ExplicitCommand c = *cmd_opt.data.t_ExplicitCommand; _mv = c; } else if (cmd_opt.tag == 0) { return al_none(); } _mv; }); // let cmd
  if (((((cmd).tag) == ((ExplicitCommand_Comment()).tag)) || (((cmd).tag) == ((ExplicitCommand_Footnote()).tag)))) {
    al_opt bal_opt = ex_scan_balanced(s, i); // let bal_opt
    struct Balanced bal = __extension__ ({ struct Balanced _mv = ((struct Balanced){ 0 }); if (bal_opt.tag == 1) { struct Balanced b = *bal_opt.data.t_Balanced; _mv = b; } else if (bal_opt.tag == 0) { return al_none(); } _mv; }); // let bal
    al_vec noargs = ((al_vec){ 0, 0, NULL }); // let noargs
    al_vec noattrs = ((al_vec){ 0, 0, NULL }); // let noattrs
    return al_some_t_HeaderParsed(((struct HeaderParsed){.cu = ((struct CommandUse){.cmd = cmd, .name_raw = name, .args = noargs, .attrs = noattrs, .content = al_some_p(al_trim(bal.text))}), .consumed = ((bal.close_idx + 1) - at)}));
  }
  al_vec args = ((al_vec){ 0, 0, NULL }); // var args
  al_vec attrs = ((al_vec){ 0, 0, NULL }); // var attrs
  while (1) {
    while (((i < strlen(s)) && ((s[i] == ' ') || (s[i] == '\t')))) {
      i += 1;
    }
    if ((i >= strlen(s))) {
      return al_none();
    }
    char c = s[i]; // let c
    if ((c == ']')) {
      i += 1;
      break;
    }
    if (fw_is_ideo_space(al_char_to_str(c))) {
      return al_none();
    }
    if (__extension__ ({ al_opt _l = (fw_normalize_in_command(al_char_to_str(c))); al_opt _r = (al_some_s(":")); (_l.tag == _r.tag && (_l.tag == 0 || strcmp(_l.data.s, _r.data.s) == 0)); })) {
      i += 1;
      continue;
    }
    al_opt _r_vp = read_value(s, i);
    if (_r_vp.tag == 0) { return _r_vp; }
    struct ValParsed vp = *_r_vp.data.t_ValParsed; // let vp
    struct Arg tok = vp.arg; // let tok
    i = (i + vp.consumed);
    int j = i; // var j
    while (((j < strlen(s)) && ((s[j] == ' ') || (s[j] == '\t')))) {
      j += 1;
    }
    int is_colon = ((j < strlen(s)) && ((s[j] == ':') || __extension__ ({ al_opt _l = (fw_normalize_in_command(al_char_to_str(s[j]))); al_opt _r = (al_some_s(":")); (_l.tag == _r.tag && (_l.tag == 0 || strcmp(_l.data.s, _r.data.s) == 0)); }))); // let is_colon
    if (is_colon) {
      j += 1;
      while (((j < strlen(s)) && ((s[j] == ' ') || (s[j] == '\t')))) {
        j += 1;
      }
      if ((j >= strlen(s))) {
        return al_none();
      }
      if (fw_is_ideo_space(al_char_to_str(s[j]))) {
        return al_none();
      }
      al_opt _r_vvp = read_value(s, j);
      if (_r_vvp.tag == 0) { return _r_vvp; }
      struct ValParsed vvp = *_r_vvp.data.t_ValParsed; // let vvp
      j = (j + vvp.consumed);
      attrs = ({ al_vec _b = al_vec_clone(attrs); al_push(&_b, sizeof(struct KVPair), &((struct KVPair){.key = arg_plain(tok), .val = vvp.arg})); _b; });
      i = j;
    } else {
      args = ({ al_vec _b = al_vec_clone(args); al_push(&_b, sizeof(struct Arg), &tok); _b; });
      al_opt ar = positional_arity(cmd); // let ar
      int over = (ar.tag == 1 ? ({ int n = ar.data.i; (args.len > n); }) : (ar.tag == 0 ? 0 : 0)); // let over
      if (over) {
        return al_none();
      }
    }
  }
  return al_some_t_HeaderParsed(((struct HeaderParsed){.cu = ((struct CommandUse){.cmd = cmd, .name_raw = name, .args = args, .attrs = attrs, .content = al_none()}), .consumed = (i - at)}));
  }
  return al_none();
}
char* inline_plain(al_vec items) {
  {
  char* out = al_strdup_lit(""); // var out
  for (size_t _i = 0; _i < items.len; _i++) {
    struct Inline it = ((struct Inline*)(items.data))[_i];
    if (it.tag == AwTxt) {
      char* s = it.data.AwTxt._0;
      {
        out = al_strcat(out, s);
      }
    }
    else if (it.tag == IBold) {
      al_vec c = it.data.IBold._0;
      {
        out = al_strcat(out, inline_plain(c));
      }
    }
    else if (it.tag == Italic) {
      al_vec c = it.data.Italic._0;
      {
        out = al_strcat(out, inline_plain(c));
      }
    }
    else if (it.tag == Strike) {
      al_vec c = it.data.Strike._0;
      {
        out = al_strcat(out, inline_plain(c));
      }
    }
    else if (it.tag == Scoped) {
      struct ExplicitCommand cmd = it.data.Scoped._0;
      al_vec c = it.data.Scoped._1;
      {
        out = al_strcat(out, inline_plain(c));
      }
    }
    else if (it.tag == CodeSpan) {
      char* s = it.data.CodeSpan._0;
      {
        out = al_strcat(out, s);
      }
    }
    else if (it.tag == Command) {
      struct CommandUse u = it.data.Command._0;
      {
        out = al_strcat(out, al_strcat(al_strcat("@[", u.name_raw), "]"));
      }
    }
    else if (it.tag == RawInline) {
      struct ExplicitCommand cmd = it.data.RawInline._0;
      al_opt lang = it.data.RawInline._1;
      char* content = it.data.RawInline._2;
      {
        out = al_strcat(out, content);
      }
    }
  }
  return out;
  }
  return "";
}
struct LineClass classify(char* line) {
  {
  if ((strlen(line) == 0)) {
    return LineClass_LcBlank();
  }
  int all_ws = 1; // var all_ws
  int qi = 0; // var qi
  while ((qi < strlen(line))) {
    if (((line[qi] != ' ') && (line[qi] != '\t'))) {
      all_ws = 0;
    }
    qi += 1;
  }
  if (all_ws) {
    return LineClass_LcBlank();
  }
  if ((((strlen(line) >= 2) && (line[0] == '@')) && (line[1] == '['))) {
    return LineClass_LcCommand();
  }
  int lead = 0; // var lead
  while (((lead < strlen(line)) && (line[lead] == ' '))) {
    lead += 1;
  }
  if ((lead <= 3)) {
    int rest_len = (strlen(line) - lead); // let rest_len
    int only_dash = (rest_len >= 3); // var only_dash
    int k = lead; // var k
    while ((k < strlen(line))) {
      if ((line[k] != '-')) {
        only_dash = 0;
      }
      k += 1;
    }
    if (only_dash) {
      return LineClass_LcDivider();
    }
  }
  if ((line[0] == '#')) {
    int n = 0; // var n
    while (((n < strlen(line)) && (line[n] == '#'))) {
      n += 1;
    }
    if ((((n <= 6) && (n < strlen(line))) && (line[n] == ' '))) {
      return LineClass_LcHeading(n);
    }
    return LineClass_LcText();
  }
  if ((line[0] == '>')) {
    int d = 0; // var d
    while (((d < strlen(line)) && (line[d] == '>'))) {
      d += 1;
    }
    if (((d < strlen(line)) && (line[d] == ' '))) {
      return LineClass_LcQuote(d);
    }
    return LineClass_LcText();
  }
  if ((((strlen(line) >= 2) && (line[0] == '-')) && (line[1] == ' '))) {
    return LineClass_LcListItem(0);
  }
  int digits = 0; // var digits
  while ((((digits < strlen(line)) && (line[digits] >= '0')) && (line[digits] <= '9'))) {
    digits += 1;
  }
  if ((((digits > 0) && (digits < strlen(line))) && (line[digits] == '.'))) {
    if ((((digits + 1) < strlen(line)) && (line[(digits + 1)] == ' '))) {
      return LineClass_LcListItem(1);
    }
  }
  return LineClass_LcText();
  }
  return ((struct LineClass){ 0 });
}
int ordered_marker_len(char* line) {
  {
  int digits = 0; // var digits
  while ((((digits < strlen(line)) && (line[digits] >= '0')) && (line[digits] <= '9'))) {
    digits += 1;
  }
  return (digits + 2);
  }
  return 0;
}
int lw_is_word_char(char* c) {
  {
  if (((strcmp(c, "0") >= 0) && (strcmp(c, "9") <= 0))) {
    return 1;
  }
  if (((strcmp(c, "a") >= 0) && (strcmp(c, "z") <= 0))) {
    return 1;
  }
  if (((strcmp(c, "A") >= 0) && (strcmp(c, "Z") <= 0))) {
    return 1;
  }
  if ((strcmp(c, "_") == 0)) {
    return 1;
  }
  return 0;
  }
  return 0;
}
struct ScanOut lw_scan_inline(char* chars, int line, al_vec diags) {
  {
  struct AtomOut ao = lw_atomize(chars, line, diags); // let ao
  return lw_resolve(ao.atoms, ao.diags, line);
  }
  return ((struct ScanOut){ 0 });
}
al_opt lw_find_char(char* s, int from, char* target) {
  {
  int i = from; // var i
  while ((i < strlen(s))) {
    if ((s[i] == (target)[0])) {
      return al_some_i(i);
    }
    i += 1;
  }
  return al_none();
  }
  return al_none();
}
al_opt lw_code_lang(struct CommandUse u) {
  {
  if ((u.args.len > 0)) {
    return al_some_s(arg_plain(((struct Arg*)(u.args.data))[0]));
  }
  return al_none();
  }
  return al_none();
}
al_opt lw_find_inline_close(char* s, int from, struct ExplicitCommand cmd) {
  {
  int i = from; // var i
  while (((i + 1) < strlen(s))) {
    if ((((s[i] == '@') && (s[(i + 1)] == '[')) && starts_with_at(s, i, "@[/"))) {
      int j = (i + 3); // var j
      while (((j < strlen(s)) && (s[j] != ']'))) {
        j += 1;
      }
      if ((j >= strlen(s))) {
        return al_none();
      }
      char* name = al_substr_ch(s, (i + 3), j); // let name
      if (close_matches(cmd, name)) {
        return al_some_t_CloseSpan(((struct CloseSpan){.start = i, .end = (j + 1)}));
      }
    }
    i += 1;
  }
  return al_none();
  }
  return al_none();
}
al_opt lw_scan_literal_object(char* s, int start) {
  {
  int i = (start + 3); // var i
  int depth = 0; // var depth
  while ((i < strlen(s))) {
    if (starts_with_at(s, i, "@[")) {
      depth += 1;
      i += 2;
      continue;
    }
    if ((s[i] == ']')) {
      if ((depth == 0)) {
        return al_some_i((i + 1));
      }
      depth -= 1;
    }
    i += 1;
  }
  return al_none();
  }
  return al_none();
}
al_opt lw_read_close_tag(char* s, int start) {
  {
  int j = (start + 3); // var j
  while (((j < strlen(s)) && (s[j] != ']'))) {
    j += 1;
  }
  if ((j >= strlen(s))) {
    return al_none();
  }
  char* name = al_substr_ch(s, (start + 3), j); // let name
  al_opt cmd_opt = lookup(name); // let cmd_opt
  return (cmd_opt.tag == 1 ? ({ struct ExplicitCommand cmd = *cmd_opt.data.t_ExplicitCommand; al_some_t_CloseHit(((struct CloseHit){.cmd = cmd, .end = (j + 1)})); }) : (cmd_opt.tag == 0 ? al_none() : al_none()));
  }
  return al_none();
}
struct AtomOut lw_atomize(char* chars, int line, al_vec diags) {
  {
  al_vec atoms = ((al_vec){ 0, 0, NULL }); // var atoms
  char* text = al_strdup_lit(""); // var text
  al_vec diags = diags; // var diags
  int i = 0; // var i
  while ((i < strlen(chars))) {
    char c = chars[i]; // let c
    if ((c == '@')) {
      if (starts_with_at(chars, i, "@@[")) {
        if ((strlen(text) > 0)) {
          atoms = ({ al_vec _b = al_vec_clone(atoms); al_push(&_b, sizeof(struct LwAtom), (__extension__ ({ struct LwAtom _t = LwAtom_AtText(text); &_t; }))); _b; });
          text = "";
        }
        al_opt lit = lw_scan_literal_object(chars, i); // let lit
        if (lit.tag == 1) {
          int end = lit.data.i;
          {
            atoms = ({ al_vec _b = al_vec_clone(atoms); al_push(&_b, sizeof(struct LwAtom), (__extension__ ({ struct LwAtom _t = LwAtom_AtText(al_substr_ch(chars, (i + 1), end)); &_t; }))); _b; });
            i = end;
          }
        }
        else if (lit.tag == 0) {
          {
            diags = ({ al_vec _b = al_vec_clone(diags); al_push(&_b, sizeof(struct Diagnostic), (__extension__ ({ struct Diagnostic _t = diag_warning("@@[ 未闭合,按字面处理", line); &_t; }))); _b; });
            text = "@";
            i += 1;
          }
        }
        continue;
      }
      if (starts_with_at(chars, i, "@[/")) {
        if ((strlen(text) > 0)) {
          atoms = ({ al_vec _b = al_vec_clone(atoms); al_push(&_b, sizeof(struct LwAtom), (__extension__ ({ struct LwAtom _t = LwAtom_AtText(text); &_t; }))); _b; });
          text = "";
        }
        al_opt hit = lw_read_close_tag(chars, i); // let hit
        if (hit.tag == 1) {
          struct CloseHit h = (*hit.data.t_CloseHit);
          {
            atoms = ({ al_vec _b = al_vec_clone(atoms); al_push(&_b, sizeof(struct LwAtom), (__extension__ ({ struct LwAtom _t = LwAtom_AtClose(h.cmd, al_substr_ch(chars, i, h.end)); &_t; }))); _b; });
            i = h.end;
          }
        }
        else if (hit.tag == 0) {
          {
            diags = ({ al_vec _b = al_vec_clone(diags); al_push(&_b, sizeof(struct Diagnostic), (__extension__ ({ struct Diagnostic _t = diag_warning("关闭标签缺少 ']',按字面处理", line); &_t; }))); _b; });
            atoms = ({ al_vec _b = al_vec_clone(atoms); al_push(&_b, sizeof(struct LwAtom), (__extension__ ({ struct LwAtom _t = LwAtom_AtText(al_substr_ch(chars, i, strlen(chars))); &_t; }))); _b; });
            i = strlen(chars);
          }
        }
        continue;
      }
      if (starts_with_at(chars, i, "@[")) {
        if ((strlen(text) > 0)) {
          atoms = ({ al_vec _b = al_vec_clone(atoms); al_push(&_b, sizeof(struct LwAtom), (__extension__ ({ struct LwAtom _t = LwAtom_AtText(text); &_t; }))); _b; });
          text = "";
        }
        al_opt hp = parse_header(chars, i); // let hp
        if (hp.tag == 1) {
          struct HeaderParsed hp2 = (*hp.data.t_HeaderParsed);
          {
            struct CommandUse u = hp2.cu; // let u
            int consumed = hp2.consumed; // let consumed
            if (((u.cmd.tag == Math) || (u.cmd.tag == Code))) {
              al_opt close_opt = lw_find_inline_close(chars, (i + consumed), u.cmd); // let close_opt
              if (close_opt.tag == 1) {
                struct CloseSpan cs = (*close_opt.data.t_CloseSpan);
                {
                  atoms = ({ al_vec _b = al_vec_clone(atoms); al_push(&_b, sizeof(struct LwAtom), (__extension__ ({ struct LwAtom _t = LwAtom_AtRawSeg(u.cmd, lw_code_lang(u), al_substr_ch(chars, (i + consumed), cs.start), al_substr_ch(chars, i, cs.end)); &_t; }))); _b; });
                  i = cs.end;
                }
              }
              else if (close_opt.tag == 0) {
                {
                  diags = ({ al_vec _b = al_vec_clone(diags); al_push(&_b, sizeof(struct Diagnostic), (__extension__ ({ struct Diagnostic _t = diag_warning(al_strcat(al_strcat("行内 ", u.name_raw), " 缺少关闭标签(应形如 @[m]…@[/m]),按字面处理"), line); &_t; }))); _b; });
                  atoms = ({ al_vec _b = al_vec_clone(atoms); al_push(&_b, sizeof(struct LwAtom), (__extension__ ({ struct LwAtom _t = LwAtom_AtText(al_substr_ch(chars, i, (i + consumed))); &_t; }))); _b; });
                  i += consumed;
                }
              }
            } else {
              int close_found = (lw_find_inline_close(chars, (i + consumed), u.cmd).tag == 1 ? 1 : (lw_find_inline_close(chars, (i + consumed), u.cmd).tag == 0 ? 0 : 0)); // let close_found
              int opens = ((is_scoped(u.cmd) || (u.cmd.tag == Cell)) && close_found); // let opens
              atoms = ({ al_vec _b = al_vec_clone(atoms); al_push(&_b, sizeof(struct LwAtom), (__extension__ ({ struct LwAtom _t = LwAtom_AtCmd(u, al_substr_ch(chars, i, (i + consumed)), opens); &_t; }))); _b; });
              i += consumed;
            }
          }
        }
        else if (hp.tag == 0) {
          struct ParseErr e = *hp.data.t_ParseErr;
          {
            struct Diagnostic mut_d = diag_error(e.msg, line); // var mut_d
            if (e.hint.tag == 1) {
              char* h = e.hint.data.s;
              {
                mut_d = diag_hint(mut_d, h);
              }
            }
            else if (e.hint.tag == 0) {
              {

              }
            }
            diags = ({ al_vec _b = al_vec_clone(diags); al_push(&_b, sizeof(struct Diagnostic), &mut_d); _b; });
            int j = (i + 2); // var j
            while (((j < strlen(chars)) && (chars[j] != ']'))) {
              j += 1;
            }
            int end = (j + 1); // let end
            int stop = ((end > strlen(chars)) ? strlen(chars) : end); // let stop
            atoms = ({ al_vec _b = al_vec_clone(atoms); al_push(&_b, sizeof(struct LwAtom), (__extension__ ({ struct LwAtom _t = LwAtom_AtText(al_substr_ch(chars, i, stop)); &_t; }))); _b; });
            i = stop;
          }
        }
        continue;
      }
      if (starts_with_at(chars, i, "@@")) {
        if ((strlen(text) > 0)) {
          atoms = ({ al_vec _b = al_vec_clone(atoms); al_push(&_b, sizeof(struct LwAtom), (__extension__ ({ struct LwAtom _t = LwAtom_AtText(text); &_t; }))); _b; });
          text = "";
        }
        atoms = ({ al_vec _b = al_vec_clone(atoms); al_push(&_b, sizeof(struct LwAtom), (__extension__ ({ struct LwAtom _t = LwAtom_AtText("@"); &_t; }))); _b; });
        i += 1;
        continue;
      }
      char* esc_text = al_strdup_lit(""); // var esc_text
      int esc_len = 0; // var esc_len
      int esc_hit = 0; // var esc_hit
      al_opt esc = escape_scan(chars, i); // let esc
      if (esc.tag == 1) {
        struct EscapeHit h = (*esc.data.t_EscapeHit);
        {
          esc_text = h.text;
          esc_len = h.consumed;
          esc_hit = 1;
        }
      }
      else if (esc.tag == 0) {
        {

        }
      }
      if (esc_hit) {
        if ((strlen(text) > 0)) {
          atoms = ({ al_vec _b = al_vec_clone(atoms); al_push(&_b, sizeof(struct LwAtom), (__extension__ ({ struct LwAtom _t = LwAtom_AtText(text); &_t; }))); _b; });
          text = "";
        }
        atoms = ({ al_vec _b = al_vec_clone(atoms); al_push(&_b, sizeof(struct LwAtom), (__extension__ ({ struct LwAtom _t = LwAtom_AtText(esc_text); &_t; }))); _b; });
        i += esc_len;
        continue;
      }
      text = al_strcat(text, "@");
      i += 1;
      continue;
    }
    if ((c == '`')) {
      if ((strlen(text) > 0)) {
        atoms = ({ al_vec _b = al_vec_clone(atoms); al_push(&_b, sizeof(struct LwAtom), (__extension__ ({ struct LwAtom _t = LwAtom_AtText(text); &_t; }))); _b; });
        text = "";
      }
      al_opt close_opt = lw_find_char(chars, (i + 1), "`"); // let close_opt
      if (close_opt.tag == 1) {
        int j = close_opt.data.i;
        {
          atoms = ({ al_vec _b = al_vec_clone(atoms); al_push(&_b, sizeof(struct LwAtom), (__extension__ ({ struct LwAtom _t = LwAtom_AtCode(al_substr_ch(chars, (i + 1), j)); &_t; }))); _b; });
          i = (j + 1);
        }
      }
      else if (close_opt.tag == 0) {
        {
          text = "`";
          i += 1;
        }
      }
      continue;
    }
    if ((((c == '*') || (c == '_')) || (c == '~'))) {
      int mlen = 0; // var mlen
      struct MarkerKind mk = MarkerKind_MkBold(); // var mk
      if (((c == '*') && starts_with_at(chars, i, "**"))) {
        mlen = 2;
        mk = MarkerKind_MkBold();
      } else {
        if (((c == '~') && starts_with_at(chars, i, "~~"))) {
          mlen = 2;
          mk = MarkerKind_MkStrike();
        } else {
          if ((c == '_')) {
            mlen = 1;
            mk = MarkerKind_MkItalic();
          }
        }
      }
      if ((mlen == 0)) {
        text = al_strcat(text, al_char_to_str(c));
        i += 1;
        continue;
      }
      if ((strlen(text) > 0)) {
        atoms = ({ al_vec _b = al_vec_clone(atoms); al_push(&_b, sizeof(struct LwAtom), (__extension__ ({ struct LwAtom _t = LwAtom_AtText(text); &_t; }))); _b; });
        text = "";
      }
      int can_open = ((i == 0) || (!lw_is_word_char(al_char_to_str(chars[(i - 1)])))); // let can_open
      int can_close = (((i + mlen) >= strlen(chars)) || (!lw_is_word_char(al_char_to_str(chars[(i + mlen)])))); // let can_close
      atoms = ({ al_vec _b = al_vec_clone(atoms); al_push(&_b, sizeof(struct LwAtom), (__extension__ ({ struct LwAtom _t = LwAtom_AtMarker(((struct MarkerAtom){.kind = mk, .literal = al_substr_ch(chars, i, (i + mlen)), .can_open = can_open, .can_close = can_close})); &_t; }))); _b; });
      i += mlen;
      continue;
    }
    text = al_strcat(text, al_char_to_str(c));
    i += 1;
  }
  if ((strlen(text) > 0)) {
    atoms = ({ al_vec _b = al_vec_clone(atoms); al_push(&_b, sizeof(struct LwAtom), (__extension__ ({ struct LwAtom _t = LwAtom_AtText(text); &_t; }))); _b; });
  }
  return ((struct AtomOut){.atoms = atoms, .diags = diags});
  }
  return ((struct AtomOut){ 0 });
}
al_vec lw_vec_set(al_vec v, int i, struct Frame f) {
  {
  al_vec out = ((al_vec){ 0, 0, NULL }); // var out
  int k = 0; // var k
  while ((k < v.len)) {
    if ((k == i)) {
      out = ({ al_vec _b = al_vec_clone(out); al_push(&_b, sizeof(struct Frame), &f); _b; });
    } else {
      out = ({ al_vec _b = al_vec_clone(out); al_push(&_b, sizeof(struct Frame), &((struct Frame*)(v.data))[k]); _b; });
    }
    k += 1;
  }
  return out;
  }
  return ((al_vec){ 0, 0, NULL });
}
al_vec lw_vec_drop_last(al_vec v) {
  {
  al_vec out = ((al_vec){ 0, 0, NULL }); // var out
  int k = 0; // var k
  while ((k < (v.len - 1))) {
    out = ({ al_vec _b = al_vec_clone(out); al_push(&_b, sizeof(struct Frame), &((struct Frame*)(v.data))[k]); _b; });
    k += 1;
  }
  return out;
  }
  return ((al_vec){ 0, 0, NULL });
}
char* lw_atom_raw(struct LwAtom a) {
  {
  if (a.tag == AtText) {
    char* s = a.data.AtText._0;
    {
      return s;
    }
  }
  else if (a.tag == AtCode) {
    char* s = a.data.AtCode._0;
    {
      return s;
    }
  }
  else if (a.tag == AtMarker) {
    struct MarkerAtom m = a.data.AtMarker._0;
    {
      return m.literal;
    }
  }
  else if (a.tag == AtCmd) {
    struct CommandUse u = a.data.AtCmd._0;
    char* raw = a.data.AtCmd._1;
    int opens = a.data.AtCmd._2;
    {
      return raw;
    }
  }
  else if (a.tag == AtClose) {
    struct ExplicitCommand cmd = a.data.AtClose._0;
    char* literal = a.data.AtClose._1;
    {
      return literal;
    }
  }
  else if (a.tag == AtRawSeg) {
    struct ExplicitCommand cmd = a.data.AtRawSeg._0;
    al_opt lang = a.data.AtRawSeg._1;
    char* content = a.data.AtRawSeg._2;
    char* raw = a.data.AtRawSeg._3;
    {
      return raw;
    }
  }
  }
  return "";
}
al_vec lw_literalize(al_vec atoms) {
  {
  char* out = al_strdup_lit(""); // var out
  for (size_t _i = 0; _i < atoms.len; _i++) {
    struct LwAtom a = ((struct LwAtom*)(atoms.data))[_i];
    out = al_strcat_own(out, lw_atom_raw(a));
  }
  return ((al_vec){ 1, sizeof(struct Inline), (struct Inline[]){ mk_text(out) } });
  }
  return ((al_vec){ 0, 0, NULL });
}
al_vec lw_merge_texts(al_vec items) {
  {
  al_vec merged = ((al_vec){ 0, 0, NULL }); // var merged
  for (size_t _i = 0; _i < items.len; _i++) {
    struct Inline it = ((struct Inline*)(items.data))[_i];
    if (it.tag == AwTxt) {
      char* s = it.data.AwTxt._0;
      {
        int did = 0; // var did
        if ((merged.len > 0)) {
          struct Inline last = ((struct Inline*)(merged.data))[(merged.len - 1)]; // let last
          if (last.tag == AwTxt) {
            char* ls = last.data.AwTxt._0;
            {
              merged = lw_vec_set_inline(merged, (merged.len - 1), mk_text(al_strcat(ls, s)));
              did = 1;
            }
          }
          else if (1) {
            {

            }
          }
        }
        if ((!did)) {
          merged = ({ al_vec _b = al_vec_clone(merged); al_push(&_b, sizeof(struct Inline), (__extension__ ({ struct Inline _t = Inline_AwTxt(s); &_t; }))); _b; });
        }
      }
    }
    else if (1) {
      struct Inline other = it;
      {
        merged = ({ al_vec _b = al_vec_clone(merged); al_push(&_b, sizeof(struct Inline), &other); _b; });
      }
    }
  }
  return merged;
  }
  return ((al_vec){ 0, 0, NULL });
}
al_vec lw_vec_set_inline(al_vec v, int i, struct Inline item) {
  {
  al_vec out = ((al_vec){ 0, 0, NULL }); // var out
  int k = 0; // var k
  while ((k < v.len)) {
    if ((k == i)) {
      out = ({ al_vec _b = al_vec_clone(out); al_push(&_b, sizeof(struct Inline), &item); _b; });
    } else {
      out = ({ al_vec _b = al_vec_clone(out); al_push(&_b, sizeof(struct Inline), &((struct Inline*)(v.data))[k]); _b; });
    }
    k += 1;
  }
  return out;
  }
  return ((al_vec){ 0, 0, NULL });
}
struct Inline mk_text(char* s) {
  {
  return Inline_AwTxt(s);
  }
  return ((struct Inline){ 0 });
}
struct ScanOut lw_resolve(al_vec atoms, al_vec diags, int line) {
  {
  al_vec frames = ((al_vec){ 1, sizeof(struct Frame), (struct Frame[]){ ((struct Frame){.kind = al_none(), .children = ((al_vec){ 0, 0, NULL })}) } }); // var frames
  al_vec diags = diags; // var diags
  int aborted = 0; // var aborted
  for (size_t _i = 0; _i < atoms.len; _i++) {
    struct LwAtom atom = ((struct LwAtom*)(atoms.data))[_i];
    if (aborted) {
      continue;
    }
    if (atom.tag == AtText) {
      char* s = atom.data.AtText._0;
      {
        int fi = (frames.len - 1); // let fi
        struct Frame f = ((struct Frame*)(frames.data))[fi]; // let f
        frames = lw_vec_set(frames, fi, ((struct Frame){.kind = f.kind, .children = ({ al_vec _b = al_vec_clone(f.children); al_push(&_b, sizeof(struct Inline), (__extension__ ({ struct Inline _t = mk_text(s); &_t; }))); _b; })}));
      }
    }
    else if (atom.tag == AtCode) {
      char* s = atom.data.AtCode._0;
      {
        int fi = (frames.len - 1); // let fi
        struct Frame f = ((struct Frame*)(frames.data))[fi]; // let f
        frames = lw_vec_set(frames, fi, ((struct Frame){.kind = f.kind, .children = ({ al_vec _b = al_vec_clone(f.children); al_push(&_b, sizeof(struct Inline), (__extension__ ({ struct Inline _t = Inline_CodeSpan(s); &_t; }))); _b; })}));
      }
    }
    else if (atom.tag == AtRawSeg) {
      struct ExplicitCommand cmd = atom.data.AtRawSeg._0;
      al_opt lang = atom.data.AtRawSeg._1;
      char* content = atom.data.AtRawSeg._2;
      char* raw = atom.data.AtRawSeg._3;
      {
        int fi = (frames.len - 1); // let fi
        struct Frame f = ((struct Frame*)(frames.data))[fi]; // let f
        frames = lw_vec_set(frames, fi, ((struct Frame){.kind = f.kind, .children = ({ al_vec _b = al_vec_clone(f.children); al_push(&_b, sizeof(struct Inline), (__extension__ ({ struct Inline _t = Inline_RawInline(cmd, lang, content); &_t; }))); _b; })}));
      }
    }
    else if (atom.tag == AtCmd) {
      struct CommandUse u = atom.data.AtCmd._0;
      char* raw = atom.data.AtCmd._1;
      int opens = atom.data.AtCmd._2;
      {
        if (opens) {
          frames = ({ al_vec _b = al_vec_clone(frames); al_push(&_b, sizeof(struct Frame), &((struct Frame){.kind = al_some_t_MarkerKind(MarkerKind_MkExplicit(u.cmd)), .children = ((al_vec){ 0, 0, NULL })})); _b; });
        } else {
          int fi = (frames.len - 1); // let fi
          struct Frame f = ((struct Frame*)(frames.data))[fi]; // let f
          frames = lw_vec_set(frames, fi, ((struct Frame){.kind = f.kind, .children = ({ al_vec _b = al_vec_clone(f.children); al_push(&_b, sizeof(struct Inline), (__extension__ ({ struct Inline _t = Inline_Command(u); &_t; }))); _b; })}));
        }
      }
    }
    else if (atom.tag == AtClose) {
      struct ExplicitCommand cmd = atom.data.AtClose._0;
      char* literal = atom.data.AtClose._1;
      {
        struct MarkerKind kind = MarkerKind_MkExplicit(cmd); // let kind
        al_opt top = ((struct Frame*)(frames.data))[(frames.len - 1)].kind; // let top
        if (__extension__ ({ al_opt _l = (top); al_opt _r = (al_some_t_MarkerKind(kind)); (_l.tag == _r.tag && (_l.tag == 0 || al_eq_MarkerKind(*_l.data.t_MarkerKind, *_r.data.t_MarkerKind))); })) {
          struct Frame f = ((struct Frame*)(frames.data))[(frames.len - 1)]; // let f
          frames = lw_vec_drop_last(frames);
          int fi = (frames.len - 1); // let fi
          struct Frame pf = ((struct Frame*)(frames.data))[fi]; // let pf
          frames = lw_vec_set(frames, fi, ((struct Frame){.kind = pf.kind, .children = ({ al_vec _b = al_vec_clone(pf.children); al_push(&_b, sizeof(struct Inline), (__extension__ ({ struct Inline _t = Inline_Scoped(cmd, f.children); &_t; }))); _b; })}));
        } else {
          if (lw_frames_contain(frames, kind)) {
            diags = ({ al_vec _b = al_vec_clone(diags); al_push(&_b, sizeof(struct Diagnostic), (__extension__ ({ struct Diagnostic _t = diag_warning("行内标记交叉(作用域次序非法),整行按字面处理", line); &_t; }))); _b; });
            aborted = 1;
          } else {
            diags = ({ al_vec _b = al_vec_clone(diags); al_push(&_b, sizeof(struct Diagnostic), (__extension__ ({ struct Diagnostic _t = diag_warning(al_strcat(al_strcat("未匹配的关闭标签 ", literal), ",按字面处理"), line); &_t; }))); _b; });
            int fi = (frames.len - 1); // let fi
            struct Frame f = ((struct Frame*)(frames.data))[fi]; // let f
            frames = lw_vec_set(frames, fi, ((struct Frame){.kind = f.kind, .children = ({ al_vec _b = al_vec_clone(f.children); al_push(&_b, sizeof(struct Inline), (__extension__ ({ struct Inline _t = mk_text(literal); &_t; }))); _b; })}));
          }
        }
      }
    }
    else if (atom.tag == AtMarker) {
      struct MarkerAtom m = atom.data.AtMarker._0;
      {
        al_opt top = ((struct Frame*)(frames.data))[(frames.len - 1)].kind; // let top
        if ((__extension__ ({ al_opt _l = (top); al_opt _r = (al_some_t_MarkerKind(m.kind)); (_l.tag == _r.tag && (_l.tag == 0 || al_eq_MarkerKind(*_l.data.t_MarkerKind, *_r.data.t_MarkerKind))); }) && m.can_close)) {
          struct Frame f = ((struct Frame*)(frames.data))[(frames.len - 1)]; // let f
          frames = lw_vec_drop_last(frames);
          int fi = (frames.len - 1); // let fi
          struct Frame pf = ((struct Frame*)(frames.data))[fi]; // let pf
          struct Inline node = (m.kind.tag == MkBold ? Inline_IBold(f.children) : (m.kind.tag == MkItalic ? Inline_Italic(f.children) : (m.kind.tag == MkStrike ? Inline_Strike(f.children) : (m.kind.tag == MkExplicit ? ({ __typeof__(m.kind.data.MkExplicit._0) cmd = m.kind.data.MkExplicit._0; Inline_Scoped(cmd, f.children); }) : ((struct Inline){ 0 }))))); // let node
          frames = lw_vec_set(frames, fi, ((struct Frame){.kind = pf.kind, .children = ({ al_vec _b = al_vec_clone(pf.children); al_push(&_b, sizeof(struct Inline), &node); _b; })}));
        } else {
          if ((m.can_close && lw_frames_contain(frames, m.kind))) {
            diags = ({ al_vec _b = al_vec_clone(diags); al_push(&_b, sizeof(struct Diagnostic), (__extension__ ({ struct Diagnostic _t = diag_warning("行内标记交叉(嵌套次序非法),整行按字面处理", line); &_t; }))); _b; });
            aborted = 1;
          } else {
            if (m.can_open) {
              frames = ({ al_vec _b = al_vec_clone(frames); al_push(&_b, sizeof(struct Frame), &((struct Frame){.kind = al_some_t_MarkerKind(m.kind), .children = ((al_vec){ 0, 0, NULL })})); _b; });
            } else {
              int fi = (frames.len - 1); // let fi
              struct Frame f = ((struct Frame*)(frames.data))[fi]; // let f
              frames = lw_vec_set(frames, fi, ((struct Frame){.kind = f.kind, .children = ({ al_vec _b = al_vec_clone(f.children); al_push(&_b, sizeof(struct Inline), (__extension__ ({ struct Inline _t = mk_text(m.literal); &_t; }))); _b; })}));
            }
          }
        }
      }
    }
  }
  if (aborted) {
    return ((struct ScanOut){.inline_a = lw_literalize(atoms), .diags = diags});
  }
  if ((frames.len > 1)) {
    diags = ({ al_vec _b = al_vec_clone(diags); al_push(&_b, sizeof(struct Diagnostic), (__extension__ ({ struct Diagnostic _t = diag_warning("行内标记未闭合,整行按字面处理", line); &_t; }))); _b; });
    return ((struct ScanOut){.inline_a = lw_literalize(atoms), .diags = diags});
  }
  return ((struct ScanOut){.inline_a = lw_merge_texts(((struct Frame*)(frames.data))[0].children), .diags = diags});
  }
  return ((struct ScanOut){ 0 });
}
int lw_frames_contain(al_vec frames, struct MarkerKind kind) {
  {
  int k = 0; // var k
  while ((k < frames.len)) {
    if (__extension__ ({ al_opt _l = (((struct Frame*)(frames.data))[k].kind); al_opt _r = (al_some_t_MarkerKind(kind)); (_l.tag == _r.tag && (_l.tag == 0 || al_eq_MarkerKind(*_l.data.t_MarkerKind, *_r.data.t_MarkerKind))); })) {
      return 1;
    }
    k += 1;
  }
  return 0;
  }
  return 0;
}
char* lex_strip_cr(char* line) {
  {
  if (((strlen(line) > 0) && (line[(strlen(line) - 1)] == '\r'))) {
    return al_substr_ch(line, 0, (strlen(line) - 1));
  }
  return line;
  }
  return "";
}
al_opt lex_close_tag_name(char* line) {
  {
  char* t = al_trim(line); // let t
  if (starts_with_at(t, 0, "@[/")) {
    if (((strlen(t) >= 4) && (t[(strlen(t) - 1)] == ']'))) {
      return al_some_s(al_substr_ch(t, 3, (strlen(t) - 1)));
    }
  }
  return al_none();
  }
  return al_none();
}
int lex_is_delimiter_cell(char* c) {
  {
  int has_dash = 0; // var has_dash
  int k = 0; // var k
  while ((k < strlen(c))) {
    char ch = c[k]; // let ch
    if ((ch == '-')) {
      has_dash = 1;
    } else {
      if ((ch != ':')) {
        return 0;
      }
    }
    k += 1;
  }
  return has_dash;
  }
  return 0;
}
struct LexOutput lex(char* src) {
  {
  al_vec blocks = ((al_vec){ 0, 0, NULL }); // var blocks
  al_vec diags = ((al_vec){ 0, 0, NULL }); // var diags
  al_opt raw = al_none(); // var raw
  int table = 0; // var table
  al_vec lines = al_split(src, "\n"); // let lines
  int lineno = 0; // var lineno
  while ((lineno < lines.len)) {
    char* line = lex_strip_cr(((char**)(lines.data))[lineno]); // let line
    lineno += 1;
    int in_raw = (raw.tag == 1 ? ({ struct ExplicitCommand cmd = *raw.data.t_ExplicitCommand; 1; }) : (raw.tag == 0 ? 0 : 0)); // let in_raw
    if (in_raw) {
      struct ExplicitCommand cmd = (raw.tag == 1 ? ({ struct ExplicitCommand c = *raw.data.t_ExplicitCommand; c; }) : (raw.tag == 0 ? ExplicitCommand_Code() : ((struct ExplicitCommand){ 0 }))); // let cmd
      al_opt close_opt = lex_close_tag_name(line); // let close_opt
      int closed = 0; // var closed
      if (close_opt.tag == 1) {
        char* name = close_opt.data.s;
        {
          if (close_matches(cmd, name)) {
            closed = 1;
          }
        }
      }
      else if (close_opt.tag == 0) {
        {

        }
      }
      if (closed) {
        blocks = ({ al_vec _b = al_vec_clone(blocks); al_push(&_b, sizeof(struct LexedBlock), &((struct LexedBlock){.line = (lineno - 1), .block = Block_RawClose(cmd)})); _b; });
        raw = al_none();
      } else {
        blocks = ({ al_vec _b = al_vec_clone(blocks); al_push(&_b, sizeof(struct LexedBlock), &((struct LexedBlock){.line = (lineno - 1), .block = Block_RawLine(line)})); _b; });
      }
      continue;
    }
    if (table) {
      char* t = al_trim(line); // let t
      if ((strcmp(t, "@[/table]") == 0)) {
        blocks = ({ al_vec _b = al_vec_clone(blocks); al_push(&_b, sizeof(struct LexedBlock), &((struct LexedBlock){.line = (lineno - 1), .block = ((struct Block){ .tag = TableClose })})); _b; });
        table = 0;
        continue;
      }
      if ((strlen(t) == 0)) {
        blocks = ({ al_vec _b = al_vec_clone(blocks); al_push(&_b, sizeof(struct LexedBlock), &((struct LexedBlock){.line = (lineno - 1), .block = ((struct Block){ .tag = Blank })})); _b; });
        continue;
      }
      if (((starts_with_at(t, 0, "|") && (t[(strlen(t) - 1)] == '|')) && (strlen(t) >= 2))) {
        char* inner = al_substr_ch(t, 1, (strlen(t) - 1)); // let inner
        al_vec cells_text = ((al_vec){ 0, 0, NULL }); // var cells_text
        char* cur = al_strdup_lit(""); // var cur
        for (size_t _i = 0; _i < strlen(inner); _i++) {
          char ch = inner[_i];
          if ((ch == '|')) {
            cells_text = ({ al_vec _b = al_vec_clone(cells_text); al_push(&_b, sizeof(char*), &(char*){ al_trim(cur) }); _b; });
            cur = "";
          } else {
            cur = al_strcat(cur, al_char_to_str(ch));
          }
        }
        cells_text = ({ al_vec _b = al_vec_clone(cells_text); al_push(&_b, sizeof(char*), &(char*){ al_trim(cur) }); _b; });
        int all_delim = 1; // var all_delim
        for (size_t _i = 0; _i < cells_text.len; _i++) {
          char* ct = ((char**)(cells_text.data))[_i];
          if ((!lex_is_delimiter_cell(ct))) {
            all_delim = 0;
          }
        }
        al_vec cells = ((al_vec){ 0, 0, NULL }); // var cells
        for (size_t _i = 0; _i < cells_text.len; _i++) {
          char* ct = ((char**)(cells_text.data))[_i];
          struct ScanOut so = lw_scan_inline(ct, (lineno - 1), diags); // let so
          diags = so.diags;
          cells = ({ al_vec _b = al_vec_clone(cells); al_push(&_b, sizeof(al_vec), &so.inline_a); _b; });
        }
        blocks = ({ al_vec _b = al_vec_clone(blocks); al_push(&_b, sizeof(struct LexedBlock), &((struct LexedBlock){.line = (lineno - 1), .block = Block_TableRow(cells, all_delim)})); _b; });
        continue;
      }
      diags = ({ al_vec _b = al_vec_clone(diags); al_push(&_b, sizeof(struct Diagnostic), (__extension__ ({ struct Diagnostic _t = diag_error("表格块内只能包含表格行(以 | 开始和结束)或 @[/table]", (lineno - 1)); &_t; }))); _b; });
      blocks = ({ al_vec _b = al_vec_clone(blocks); al_push(&_b, sizeof(struct LexedBlock), &((struct LexedBlock){.line = (lineno - 1), .block = Block_Paragraph(({ al_vec __v = { 0, sizeof(struct Inline), NULL }; { struct Inline _e0 = Inline_AwTxt(line); al_push(&__v, sizeof(_e0), &_e0); } __v; }))})); _b; });
      continue;
    }
    struct LineClass cls = classify(line); // let cls
    if (cls.tag == LcBlank) {
      {
        blocks = ({ al_vec _b = al_vec_clone(blocks); al_push(&_b, sizeof(struct LexedBlock), &((struct LexedBlock){.line = (lineno - 1), .block = ((struct Block){ .tag = Blank })})); _b; });
      }
    }
    else if (cls.tag == LcDivider) {
      {
        blocks = ({ al_vec _b = al_vec_clone(blocks); al_push(&_b, sizeof(struct LexedBlock), &((struct LexedBlock){.line = (lineno - 1), .block = ((struct Block){ .tag = Divider })})); _b; });
      }
    }
    else if (cls.tag == LcHeading) {
      int level = cls.data.LcHeading._0;
      {
        struct ScanOut so = lw_scan_inline(al_substr_ch(line, (level + 1), strlen(line)), (lineno - 1), diags); // let so
        diags = so.diags;
        blocks = ({ al_vec _b = al_vec_clone(blocks); al_push(&_b, sizeof(struct LexedBlock), &((struct LexedBlock){.line = (lineno - 1), .block = Block_Heading(level, so.inline_a)})); _b; });
      }
    }
    else if (cls.tag == LcQuote) {
      int depth = cls.data.LcQuote._0;
      {
        struct ScanOut so = lw_scan_inline(al_substr_ch(line, (depth + 1), strlen(line)), (lineno - 1), diags); // let so
        diags = so.diags;
        blocks = ({ al_vec _b = al_vec_clone(blocks); al_push(&_b, sizeof(struct LexedBlock), &((struct LexedBlock){.line = (lineno - 1), .block = Block_Quote(depth, so.inline_a)})); _b; });
      }
    }
    else if (cls.tag == LcListItem) {
      int ordered = cls.data.LcListItem._0;
      {
        int skip = (ordered ? ordered_marker_len(line) : 2); // let skip
        struct ScanOut so = lw_scan_inline(al_substr_ch(line, skip, strlen(line)), (lineno - 1), diags); // let so
        diags = so.diags;
        blocks = ({ al_vec _b = al_vec_clone(blocks); al_push(&_b, sizeof(struct LexedBlock), &((struct LexedBlock){.line = (lineno - 1), .block = Block_ListItem(ordered, so.inline_a)})); _b; });
      }
    }
    else if (cls.tag == LcCommand) {
      {
        struct CmdOutcome co = lex_command_line(line, (lineno - 1), diags, raw, table); // let co
        for (size_t _i = 0; _i < co.blocks.len; _i++) {
          struct LexedBlock nb = ((struct LexedBlock*)(co.blocks.data))[_i];
          blocks = ({ al_vec _b = al_vec_clone(blocks); al_push(&_b, sizeof(struct LexedBlock), &nb); _b; });
        }
        diags = co.diags;
        raw = co.raw;
        table = co.table;
      }
    }
    else if (cls.tag == LcText) {
      {
        struct ScanOut so = lw_scan_inline(line, (lineno - 1), diags); // let so
        diags = so.diags;
        blocks = ({ al_vec _b = al_vec_clone(blocks); al_push(&_b, sizeof(struct LexedBlock), &((struct LexedBlock){.line = (lineno - 1), .block = Block_Paragraph(so.inline_a)})); _b; });
      }
    }
  }
  int last = lines.len; // let last
  if (raw.tag == 1) {
    struct ExplicitCommand cmd = (*raw.data.t_ExplicitCommand);
    {
      diags = ({ al_vec _b = al_vec_clone(diags); al_push(&_b, sizeof(struct Diagnostic), (__extension__ ({ struct Diagnostic _t = diag_error("Raw Block 未闭合到文件结尾", last); &_t; }))); _b; });
    }
  }
  else if (raw.tag == 0) {
    {

    }
  }
  if (table) {
    diags = ({ al_vec _b = al_vec_clone(diags); al_push(&_b, sizeof(struct Diagnostic), (__extension__ ({ struct Diagnostic _t = diag_error("@[table] 未闭合到文件结尾", last); &_t; }))); _b; });
  }
  return ((struct LexOutput){.blocks = blocks, .diags = diags});
  }
  return ((struct LexOutput){ 0 });
}
struct CmdOutcome lex_command_line(char* chars, int lineno, al_vec diags, al_opt raw, int table) {
  {
  al_vec blocks = ((al_vec){ 0, 0, NULL }); // var blocks
  al_vec diags = diags; // var diags
  al_opt hp = parse_header(chars, 0); // let hp
  if (hp.tag == 0) {
    struct ParseErr e = *hp.data.t_ParseErr;
    {
      struct Diagnostic mut_d = diag_error(e.msg, lineno); // var mut_d
      if (e.hint.tag == 1) {
        char* h = e.hint.data.s;
        {
          mut_d = diag_hint(mut_d, h);
        }
      }
      else if (e.hint.tag == 0) {
        {

        }
      }
      diags = ({ al_vec _b = al_vec_clone(diags); al_push(&_b, sizeof(struct Diagnostic), &mut_d); _b; });
      blocks = ({ al_vec _b = al_vec_clone(blocks); al_push(&_b, sizeof(struct LexedBlock), &((struct LexedBlock){.line = lineno, .block = Block_Paragraph(({ al_vec __v = { 0, sizeof(struct Inline), NULL }; { struct Inline _e0 = Inline_AwTxt(chars); al_push(&__v, sizeof(_e0), &_e0); } __v; }))})); _b; });
      return ((struct CmdOutcome){.blocks = blocks, .diags = diags, .raw = raw, .table = table});
    }
  }
  else if (hp.tag == 1) {
    struct HeaderParsed hp2 = (*hp.data.t_HeaderParsed);
    {
      struct CommandUse u = hp2.cu; // let u
      int consumed = hp2.consumed; // let consumed
      if ((u.cmd.tag == Table)) {
        blocks = ({ al_vec _b = al_vec_clone(blocks); al_push(&_b, sizeof(struct LexedBlock), &((struct LexedBlock){.line = lineno, .block = Block_TableOpen(u)})); _b; });
        return ((struct CmdOutcome){.blocks = blocks, .diags = diags, .raw = raw, .table = 1});
      }
      if (is_raw_block(u.cmd)) {
        int header_empty = (u.content.tag == 0 ? 1 : (u.content.tag == 1 ? ({ char* c = u.content.data.s; (strlen(c) == 0); }) : 0)); // let header_empty
        char* rest = al_substr_ch(chars, consumed, strlen(chars)); // let rest
        if ((header_empty && (al_chars_len(al_trim(rest)) == 0))) {
          al_opt lang = al_none(); // var lang
          if (((u.cmd.tag == Code) && (u.args.len > 0))) {
            lang = al_some_s(arg_plain(((struct Arg*)(u.args.data))[0]));
          }
          blocks = ({ al_vec _b = al_vec_clone(blocks); al_push(&_b, sizeof(struct LexedBlock), &((struct LexedBlock){.line = lineno, .block = Block_RawOpen(u.cmd, lang)})); _b; });
          return ((struct CmdOutcome){.blocks = blocks, .diags = diags, .raw = al_some_t_ExplicitCommand(u.cmd), .table = table});
        }
      }
      struct ScanOut so = lw_scan_inline(chars, lineno, diags); // let so
      int is_single = 0; // var is_single
      al_opt single = al_none(); // var single
      if ((so.inline_a.len == 1)) {
        if (((struct Inline*)(so.inline_a.data))[0].tag == Command) {
          struct CommandUse cu = ((struct Inline*)(so.inline_a.data))[0].data.Command._0;
          {
            is_single = 1;
            single = al_some_t_CommandUse(cu);
          }
        }
        else if (1) {
          {

          }
        }
      }
      if (single.tag == 1) {
        struct CommandUse cu = (*single.data.t_CommandUse);
        {
          blocks = ({ al_vec _b = al_vec_clone(blocks); al_push(&_b, sizeof(struct LexedBlock), &((struct LexedBlock){.line = lineno, .block = Block_Object(cu)})); _b; });
        }
      }
      else if (single.tag == 0) {
        {
          blocks = ({ al_vec _b = al_vec_clone(blocks); al_push(&_b, sizeof(struct LexedBlock), &((struct LexedBlock){.line = lineno, .block = Block_Paragraph(so.inline_a)})); _b; });
        }
      }
      return ((struct CmdOutcome){.blocks = blocks, .diags = diags, .raw = raw, .table = table});
    }
  }
  }
  return ((struct CmdOutcome){ 0 });
}
al_opt span_new(int start, int end) {
  {
  if ((start <= end)) {
    return al_some_t_SourceSpan(((struct SourceSpan){.start = start, .end = end}));
  }
  return al_none();
  }
  return al_none();
}
al_opt span_after_edit(struct SourceSpan s, int e_start, int e_end, int new_len) {
  {
  if ((e_end <= s.start)) {
    int delta = (new_len - (e_end - e_start)); // let delta
    return span_new((s.start + delta), (s.end + delta));
  }
  if ((e_start >= s.end)) {
    return al_some_t_SourceSpan(s);
  }
  return al_none();
  }
  return al_none();
}
int label_char_ok(char* c) {
  {
  if (((strcmp(c, "0") >= 0) && (strcmp(c, "9") <= 0))) {
    return 1;
  }
  if (((strcmp(c, "a") >= 0) && (strcmp(c, "z") <= 0))) {
    return 1;
  }
  if (((strcmp(c, "A") >= 0) && (strcmp(c, "Z") <= 0))) {
    return 1;
  }
  if (((strcmp(c, "_") == 0) || (strcmp(c, "-") == 0))) {
    return 1;
  }
  if ((strcmp(c, "\\x7f") > 0)) {
    return 1;
  }
  return 0;
  }
  return 0;
}
al_opt label_parse(char* s) {
  {
  if ((strlen(s) == 0)) {
    return al_none();
  }
  int i = 0; // var i
  while ((i < strlen(s))) {
    if ((!label_char_ok(al_char_to_str(s[i])))) {
      return al_none();
    }
    i += 1;
  }
  return al_some_s(s);
  }
  return al_none();
}
struct TextPatch patch_invert(struct TextPatch p) {
  {
  return ((struct TextPatch){.start = p.start, .old = p.new, .new = p.old});
  }
  return ((struct TextPatch){ 0 });
}
struct Buffer buffer_new(char* s) {
  {
  return ((struct Buffer){.content = s});
  }
  return ((struct Buffer){ 0 });
}
al_opt buffer_insert(struct Buffer b, int start, char* s) {
  {
  if (((start < 0) || (start > strlen(b.content)))) {
    return al_none();
  }
  struct Buffer nl = ((struct Buffer){.content = al_strcat(al_strcat(al_substr_ch(b.content, 0, start), s), al_substr_ch(b.content, start, strlen(b.content)))}); // let nl
  return al_some_t_BufWithPatch(((struct BufWithPatch){.buf = nl, .patch = ((struct TextPatch){.start = start, .old = "", .new = s})}));
  }
  return al_none();
}
al_opt buffer_delete(struct Buffer b, int start, int end) {
  {
  if ((((start < 0) || (end > strlen(b.content))) || (start > end))) {
    return al_none();
  }
  char* old = al_substr_ch(b.content, start, end); // let old
  struct Buffer nl = ((struct Buffer){.content = al_strcat(al_substr_ch(b.content, 0, start), al_substr_ch(b.content, end, strlen(b.content)))}); // let nl
  return al_some_t_BufWithPatch(((struct BufWithPatch){.buf = nl, .patch = ((struct TextPatch){.start = start, .old = old, .new = ""})}));
  }
  return al_none();
}
al_opt buffer_apply(struct Buffer b, struct TextPatch p) {
  {
  if (((p.start < 0) || ((p.start + strlen(p.old)) > strlen(b.content)))) {
    return al_none();
  }
  if ((strcmp(al_substr_ch(b.content, p.start, (p.start + strlen(p.old))), p.old) != 0)) {
    return al_none();
  }
  return al_some_t_Buffer(((struct Buffer){.content = al_strcat(al_strcat(al_substr_ch(b.content, 0, p.start), p.new), al_substr_ch(b.content, (p.start + strlen(p.old)), strlen(b.content)))}));
  }
  return al_none();
}
int main(int _argc, char** _argv) {
  al_cli_args = (al_vec){ 0, sizeof(char*), NULL };
  for (int _ai = 1; _ai < _argc; _ai++) { char* _av = _argv[_ai]; al_push(&al_cli_args, sizeof(char*), (void*)&_av); }
  {
  printf("%s\n", "Awen 原型词法器(Aine 移植版,核心规范 v0.4)");
  char* src = read_file("corpus/all_syntax.awen"); // let src
  if ((strstr(src, "[read_file error") != NULL)) {
    printf("%s\n", al_strcat(al_strcat("[corpus 加载失败: ", src), "](应以 awen-proto 为工作目录运行)"));
    return 0;
  }
  struct LexOutput out = lex(src); // let out
  int errors = 0; // var errors
  int warnings = 0; // var warnings
  for (size_t _i = 0; _i < out.diags.len; _i++) {
    struct Diagnostic d = ((struct Diagnostic*)(out.diags.data))[_i];
    if ((d.severity.tag == SevError)) {
      errors += 1;
      printf("%s\n", al_strcat(al_strcat(al_strcat("  错误(行 ", al_num((d.line + 1))), "): "), d.message));
    } else {
      warnings += 1;
    }
  }
  int objects = 0; // var objects
  int headings = 0; // var headings
  int raws = 0; // var raws
  int rows = 0; // var rows
  for (size_t _i = 0; _i < out.blocks.len; _i++) {
    struct LexedBlock b = ((struct LexedBlock*)(out.blocks.data))[_i];
    if (b.block.tag == Object) {
      struct CommandUse u = b.block.data.Object._0;
      {
        objects += 1;
      }
    }
    else if (b.block.tag == Heading) {
      int level = b.block.data.Heading._0;
      al_vec inline_a = b.block.data.Heading._1;
      {
        headings += 1;
      }
    }
    else if (b.block.tag == RawOpen) {
      struct ExplicitCommand cmd = b.block.data.RawOpen._0;
      al_opt lang = b.block.data.RawOpen._1;
      {
        raws += 1;
      }
    }
    else if (b.block.tag == TableRow) {
      al_vec cells = b.block.data.TableRow._0;
      int delimiter = b.block.data.TableRow._1;
      {
        rows += 1;
      }
    }
    else if (1) {
      {

      }
    }
  }
  printf("%s\n", al_strcat(al_strcat(al_strcat(al_strcat(al_strcat(al_strcat(al_strcat(al_strcat(al_strcat(al_strcat("corpus/all_syntax.awen: ", al_num(out.blocks.len)), " 块, "), al_num(headings)), " 标题, "), al_num(objects)), " 独立对象, "), al_num(raws)), " Raw 块, "), al_num(rows)), " 表格行"));
  printf("%s\n", al_strcat(al_strcat(al_strcat(al_strcat("诊断: ", al_num(errors)), " 错误, "), al_num(warnings)), " 告警(告警应全部来自转义演示区的预期字面回退)"));
  if ((errors == 0)) {
    printf("%s\n", "词法验收:通过 ✓");
  } else {
    printf("%s\n", "词法验收:失败 ✗");
  }
  }
  return 0;
}
