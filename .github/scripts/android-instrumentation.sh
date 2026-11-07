#!/usr/bin/env bash
adb wait-for-device
adb shell getprop sys.boot_completed
adb logcat -c

gradle --no-daemon :app:connectedDebugAndroidTest
test_status=$?

adb logcat -d -v threadtime > "$GITHUB_WORKSPACE/android-logcat.txt" || true
adb shell dumpsys activity processes > "$GITHUB_WORKSPACE/android-processes.txt" || true

exit "$test_status"
