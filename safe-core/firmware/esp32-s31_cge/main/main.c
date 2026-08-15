// esp32-s31_cge/main/main.c

#include <stdio.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "esp_log.h"
#include "driver/spi_master.h"
#include "../../shared/safecore_protocol.h"

static const char *TAG = "CGE";

void app_main() {
    ESP_LOGI(TAG, "CGE Started");
}
