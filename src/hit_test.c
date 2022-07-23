// Based on https://fossies.org/linux/SDL2/test/testhittesting.c

#include <stdio.h>
#include <SDL2/SDL.h>


typedef struct HitTestData {
    SDL_Rect ** add;
    int add_len;
    SDL_Rect ** sub;
    int sub_len;
} HitTestData;


SDL_HitTestResult hitTest(__attribute__((unused)) SDL_Window *window, const SDL_Point *pt, void *data) {
    
    HitTestData hit_test_data = *((HitTestData*) data);

    int i = 0;
    int j = 0;

    for (i = 0; i < hit_test_data.add_len; i++) {
        if (SDL_PointInRect(pt, hit_test_data.add[i])) {
            for (j = 0; j < hit_test_data.sub_len; j++) {
                if (SDL_PointInRect(pt, hit_test_data.sub[j])) {
                    return SDL_HITTEST_NORMAL;
                }
            }
            return SDL_HITTEST_DRAGGABLE;
        }
    }

    // if (hit_test_data.sub_len > 0) {
    //     SDL_Rect rect = *hit_test_data.sub[0];
    //     SDL_Log("x: %i, y: %i, w: %i, h: %i", rect.x, rect.y, rect.w, rect.h);
    //     SDL_Log("length: %i", hit_test_data.add_len);
    // }

    return SDL_HITTEST_NORMAL;
}
