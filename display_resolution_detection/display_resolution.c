#include <stdio.h>

/**
 * Determination a platform of an operation system
 * Fully supported supported only GNU GCC/G++, partially on Clang/LLVM
 */

#define PLATFORM_WINDOWS 1
#define PLATFORM_LINUX 2
#define PLATFORM_MACOS 3
#define PLATFORM_UNKNOWN -1

#if defined(_WIN32)
    #define PLATFORM 1 // Windows
    #include <windows.h>
#elif defined(_WIN64)
    #define PLATFORM 1 // Windows
    #include <windows.h>
#elif defined(__CYGWIN__) && !defined(_WIN32)
    #define PLATFORM 1 // Windows (Cygwin POSIX under Microsoft Window)
    #include <windows.h>
#elif defined(__linux__)
    #define PLATFORM 2 // Debian, Ubuntu, Gentoo, Fedora, openSUSE, RedHat, Centos and other
    #include <stdio.h>
    #include <string.h>
    #include <stdlib.h>
#elif defined(__APPLE__) && defined(__MACH__) // Apple OSX and iOS (Darwin)
    #include <TargetConditionals.h>
    #if TARGET_OS_MAC == 1
        #define PLATFORM 3 // Apple OSX
        #include <CoreGraphics/CGDisplayConfiguration.h>
    #endif
#else
    #define PLATFORM -1
#endif

int fn_lunux(int* width, int* height)
{
    char *array[8];
    char screen_size[64];
    char* token = NULL;

    FILE *cmd = popen("xdpyinfo | awk '/dimensions/ {print $2}'", "r");

    if (!cmd)
        return 0;

    while (fgets(screen_size, sizeof(screen_size), cmd) != NULL);
    pclose(cmd);

    token = strtok(screen_size, "x\n");

    if (!token)
        return -1;

    for (unsigned short i = 0; token != NULL; ++i) {
        array[i] = token;
        token = strtok(NULL, "x\n");
    }
    *width = atoi(array[0]);
    *height = atoi(array[1]);

    return 0;
}

int main(int argc, char *argv[]) {
    
    int height, width, err;

    switch (PLATFORM)
    {
        case PLATFORM_WINDOWS:
                height = GetSystemMetrics(SM_CYSCREEN);
                width = GetSystemMetrics(SM_CXSCREEN);
            break;
        case PLATFORM_LINUX:
                err = fn_lunux(&width, &height);
                if(err) return err;
            break;

        case PLATFORM_MACOS:
            width = CGDisplayPixelsWide(CGMainDisplayID());
            height = CGDisplayPixelsHigh(CGMainDisplayID());
            break;
        
        default:
            return PLATFORM_UNKNOWN;
    } 

    printf("width: %d height: %d\n", width, height);

    system("pause");
    return 0;
}

