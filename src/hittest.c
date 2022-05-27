// Based on https://fossies.org/linux/SDL2/test/testhittesting.c

#include <stdio.h>
#include <SDL2/SDL.h>

// #define RESIZE_BORDER 20

const SDL_Rect drag_areas[] = {
    { 0, 0, 200, 200 },
    // { 200, 70, 100, 100 },
    // { 400, 90, 100, 100 }
};

// static int screenSize = 0

static const SDL_Rect *areas = drag_areas;
static int numareas = SDL_arraysize(drag_areas);

SDL_HitTestResult hitTest(SDL_Window *window, const SDL_Point *pt, void *data) {
    int i;
    // int w, h;

    for (i = 0; i < numareas; i++) {
        if (SDL_PointInRect(pt, &areas[i])) {
            // SDL_Log("HIT-TEST: DRAGGABLE\n");
            return SDL_HITTEST_DRAGGABLE;
        }
    }

    // SDL_GetWindowSize(window, &w, &h);

    // #define REPORT_RESIZE_HIT(name) { \
    //     SDL_Log("HIT-TEST: RESIZE_" #name "\n"); \
    //     return SDL_HITTEST_RESIZE_##name; \
    // }

    // if (pt->x < RESIZE_BORDER && pt->y < RESIZE_BORDER) {
    //     REPORT_RESIZE_HIT(TOPLEFT);
    // } else if (pt->x > RESIZE_BORDER && pt->x < w - RESIZE_BORDER && pt->y < RESIZE_BORDER) {
    //     REPORT_RESIZE_HIT(TOP);
    // } else if (pt->x > w - RESIZE_BORDER && pt->y < RESIZE_BORDER) {
    //     REPORT_RESIZE_HIT(TOPRIGHT);
    // } else if (pt->x > w - RESIZE_BORDER && pt->y > RESIZE_BORDER && pt->y < h - RESIZE_BORDER) {
    //     REPORT_RESIZE_HIT(RIGHT);
    // } else if (pt->x > w - RESIZE_BORDER && pt->y > h - RESIZE_BORDER) {
    //     REPORT_RESIZE_HIT(BOTTOMRIGHT);
    // } else if (pt->x < w - RESIZE_BORDER && pt->x > RESIZE_BORDER && pt->y > h - RESIZE_BORDER) {
    //     REPORT_RESIZE_HIT(BOTTOM);
    // } else if (pt->x < RESIZE_BORDER && pt->y > h - RESIZE_BORDER) {
    //     REPORT_RESIZE_HIT(BOTTOMLEFT);
    // } else if (pt->x < RESIZE_BORDER && pt->y < h - RESIZE_BORDER && pt->y > RESIZE_BORDER) {
    //     REPORT_RESIZE_HIT(LEFT);
    // }

    // SDL_Log("HIT-TEST: NORMAL\n");
    return SDL_HITTEST_NORMAL;
}