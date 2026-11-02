package com.mitosv.mitosis.bridge.telemetry;

import net.minecraft.server.MinecraftServer;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public class ServerTelemetryReporter {
    private static final Logger LOGGER = LoggerFactory.getLogger("MitosisBridge/Telemetry");
    private static long lastReportTime = 0;

    public static void onServerTick(MinecraftServer server) {
        long now = System.currentTimeMillis();
        if (now - lastReportTime >= 1000) {
            lastReportTime = now;
            float mspt = server.getAverageTickTime();
            float tps = Math.min(20.0f, 1000.0f / Math.max(mspt, 1.0f));
            int botCount = 0;
            for (var player : server.getPlayerManager().getPlayerList()) {
                if (player.getName().getString().startsWith("bot")) {
                    botCount++;
                }
            }
            if (botCount > 0) {
                LOGGER.debug("Server Telemetry: TPS={:.2f}, MSPT={:.2f}ms, Active Bots={}", tps, mspt, botCount);
            }
        }
    }
}
