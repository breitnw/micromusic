// Based on https://fossies.org/linux/SDL2/test/testhittesting.c

#include <stdio.h>
#include <SDL2/SDL.h>


typedef struct HitTestData {
    SDL_Rect** add;
    int add_len;
    SDL_Rect** sub;
    int sub_len;
} HitTestData;


SDL_HitTestResult hitTest(__attribute__((unused)) SDL_Window* window, const SDL_Point* pt, void* data) {
    
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

    return SDL_HITTEST_NORMAL;
}


float get_wheel_y(SDL_Event* event_ptr) {
    SDL_Event event = *event_ptr;
    if(event.type == SDL_MOUSEWHEEL) {
        return event.wheel.preciseY;
    }
    if(event.type == SDL_DOLLARGESTURE) {
        return 100000.0;
    }
    return 0.0;
}