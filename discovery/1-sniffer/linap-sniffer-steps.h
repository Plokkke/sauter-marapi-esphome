#pragma once

// Guided capture of the Linap bus: one radiator action per step, then hands off while the bus settles.
// Edit freely: label (logged as the action marker) and wait in seconds.
struct CaptureStep {
  const char *label;
  int wait_s;
};

static const CaptureStep CAPTURE_STEPS[] = {
    {"baseline: touch nothing", 60},
    {"power on hard (wall switch ON)", 90},
    {"setpoint +0.5", 30},
    {"setpoint +0.5", 30},
    {"setpoint -0.5", 30},
    {"setpoint -0.5", 30},
    {"boost on", 60},
    {"boost off", 30},
    {"mode eco", 30},
    {"mode frost protection", 30},
    {"mode comfort", 30},
    {"keypad lock on", 30},
    {"keypad lock off", 30},
    {"menu: browse without changing anything", 30},
    {"power off soft (on/off key)", 45},
    {"power on soft (on/off key)", 60},
    {"power off hard (wall switch OFF)", 30},
};

static const int CAPTURE_STEP_COUNT = sizeof(CAPTURE_STEPS) / sizeof(CAPTURE_STEPS[0]);
