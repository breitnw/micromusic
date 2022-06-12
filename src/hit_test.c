// Based on https://fossies.org/linux/SDL2/test/testhittesting.c

#include <stdio.h>
#include <SDL2/SDL.h>

// #define RESIZE_BORDER 20

const SDL_Rect drag_areas[] = {
    { 0, 20, 200, 180 },
    { 20, 0, 140, 20 },
};

static const SDL_Rect *areas = drag_areas;
static int numareas = SDL_arraysize(drag_areas);

SDL_HitTestResult hitTest(SDL_Window *window, const SDL_Point *pt, void *data) {
    int i;

    for (i = 0; i < numareas; i++) {
        if (SDL_PointInRect(pt, &areas[i])) {
            return SDL_HITTEST_DRAGGABLE;
        }
    }

    return SDL_HITTEST_NORMAL;
}