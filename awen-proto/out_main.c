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
void al_push(al_vec* v, size_t es, void* elem) {
  if (v->elem_size == 0) v->elem_size = es;
  v->data = realloc(v->data, (v->len + 1) * v->elem_size);
  if (v->data && elem) memcpy((char*)v->data + v->len * v->elem_size, elem, v->elem_size);
  v->len++;
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
static al_vec al_split(char* s, char* sep) { al_vec r = { 0, 0, NULL }; size_t sl = strlen(sep); if (sl == 0) { for (size_t i = 0; i < strlen(s); i++) { char* p = (char*)malloc(2); p[0] = s[i]; p[1] = 0; al_push(&r, sizeof(char*), &p); } } else { char* cur = s; while (1) { char* hit = strstr(cur, sep); if (!hit) { al_push(&r, sizeof(char*), &(char*){ cur }); break; } size_t n = (size_t)(hit - cur); char* p = (char*)malloc(n + 1); memcpy(p, cur, n); p[n] = 0; al_push(&r, sizeof(char*), &p); cur = hit + sl; } return r; } }
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
typedef struct { int tag; union { int i; double f; char* s; al_vec v; void* p; } data; } al_opt;
#define al_some_i(x) ((al_opt){ .tag = 1, .data.i = (x) })
#define al_some_f(x) ((al_opt){ .tag = 1, .data.f = (x) })
#define al_some_s(x) ((al_opt){ .tag = 1, .data.s = (x) })
#define al_some_v(x) ((al_opt){ .tag = 1, .data.v = (x) })
#define al_some_p(x) ((al_opt){ .tag = 1, .data.p = (void*)(x) })
#define al_none() ((al_opt){ .tag = 0 })
int main(int _argc, char** _argv);
/* unsupported stmt */
/* unsupported stmt */
/* unsupported stmt */
/* unsupported stmt */
/* unsupported stmt */
/* unsupported stmt */
/* unsupported stmt */
/* unsupported stmt */
int main(int _argc, char** _argv) {
  al_cli_args = (al_vec){ 0, sizeof(char*), NULL };
  for (int _ai = 1; _ai < _argc; _ai++) { char* _av = _argv[_ai]; al_push(&al_cli_args, sizeof(char*), (void*)&_av); }
  {
  printf("%s\n", "Awen 原型词法器(Aine 移植版,核心规范 v0.4)");
  char* src = read_file("corpus/all_syntax.awen"); // let src
  if ((strstr(src, "[read_file error") != NULL)) {
    printf("%s\n", al_strcat(al_strcat("[corpus 加载失败: ", src), "](应以 awen-proto 为工作目录运行)"));
    return;
  }
  int out = lex(src); // let out
  int errors = 0; // var errors
  int warnings = 0; // var warnings
  for (size_t _i = 0; _i < strlen(out.diags); _i++) {
    char d = out.diags[_i];
    if ((d.severity.tag == SevError)) {
      errors += 1;
      printf("%s\n", al_strcat(al_strcat(al_strcat("  错误(行 ", al_num((d.line + 1))), "): "), al_num(d.message)));
    } else {
      warnings += 1;
    }
  }
  int objects = 0; // var objects
  int headings = 0; // var headings
  int raws = 0; // var raws
  int rows = 0; // var rows
  for (size_t _i = 0; _i < strlen(out.blocks); _i++) {
    char b = out.blocks[_i];
    if (b.block.tag == Object) {
      auto u = b.block.data.Object._0;
      {
        objects += 1;
      }
    }
    else if (b.block.tag == Heading) {
      auto level = b.block.data.Heading._0;
      auto inline = b.block.data.Heading._1;
      {
        headings += 1;
      }
    }
    else if (b.block.tag == RawOpen) {
      auto cmd = b.block.data.RawOpen._0;
      auto lang = b.block.data.RawOpen._1;
      {
        raws += 1;
      }
    }
    else if (b.block.tag == TableRow) {
      auto cells = b.block.data.TableRow._0;
      auto delimiter = b.block.data.TableRow._1;
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
