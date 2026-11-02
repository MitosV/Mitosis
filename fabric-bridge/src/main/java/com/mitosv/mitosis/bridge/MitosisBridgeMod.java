package com.mitosv.mitosis.bridge;

import com.mitosv.mitosis.bridge.command.MitosisCommands;
import com.mitosv.mitosis.bridge.telemetry.ServerTelemetryReporter;
import net.fabricmc.api.DedicatedServerModInitializer;
import net.fabricmc.fabric.api.command.v2.CommandRegistrationCallback;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public class MitosisBridgeMod implements DedicatedServerModInitializer {
    public static final String MOD_ID = "mitosis-bridge";
    public static final Logger LOGGER = LoggerFactory.getLogger(MOD_ID);

    @Override
    public void onInitializeServer() {
        LOGGER.info("Mitosis Bridge Fabric Mod 1.21.1 Initialized (Protocol 767)");

        CommandRegistrationCallback.EVENT.register(MitosisCommands::register);
        ServerTickEvents.END_SERVER_TICK.register(ServerTelemetryReporter::onServerTick);
    }
}
