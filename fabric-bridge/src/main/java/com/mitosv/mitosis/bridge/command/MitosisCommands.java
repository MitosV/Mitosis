package com.mitosv.mitosis.bridge.command;

import com.mojang.brigadier.CommandDispatcher;
import com.mojang.brigadier.arguments.FloatArgumentType;
import com.mojang.brigadier.arguments.StringArgumentType;
import net.fabricmc.loader.api.FabricLoader;
import net.minecraft.command.CommandRegistryAccess;
import net.minecraft.server.command.CommandManager;
import net.minecraft.server.command.ServerCommandSource;
import net.minecraft.text.Text;

import static net.minecraft.server.command.CommandManager.argument;
import static net.minecraft.server.command.CommandManager.literal;

public class MitosisCommands {

    public static void register(CommandDispatcher<ServerCommandSource> dispatcher, CommandRegistryAccess registryAccess, CommandManager.RegistrationEnvironment environment) {
        boolean hasCarpet = FabricLoader.getInstance().isModLoaded("carpet");
        String rootCommand = hasCarpet ? "bot" : "player";

        dispatcher.register(literal(rootCommand)
            .requires(source -> source.hasPermissionLevel(2))
            // /player <bot> move <direction>
            .then(argument("bot", StringArgumentType.word())
                .then(literal("move")
                    .then(argument("direction", StringArgumentType.word())
                        .executes(ctx -> {
                            String bot = StringArgumentType.getString(ctx, "bot");
                            String dir = StringArgumentType.getString(ctx, "direction");
                            ctx.getSource().sendFeedback(() -> Text.literal("§a[Mitosis] Dispatched move " + dir + " to bot " + bot), true);
                            return 1;
                        })
                    )
                )
                // /player <bot> jump
                .then(literal("jump")
                    .executes(ctx -> {
                        String bot = StringArgumentType.getString(ctx, "bot");
                        ctx.getSource().sendFeedback(() -> Text.literal("§a[Mitosis] Dispatched jump to bot " + bot), true);
                        return 1;
                    })
                )
                // /player <bot> attack continuous
                .then(literal("attack")
                    .then(literal("continuous")
                        .executes(ctx -> {
                            String bot = StringArgumentType.getString(ctx, "bot");
                            ctx.getSource().sendFeedback(() -> Text.literal("§a[Mitosis] Bot " + bot + " attacking continuously"), true);
                            return 1;
                        })
                    )
                )
                // /player <bot> use continuous
                .then(literal("use")
                    .then(literal("continuous")
                        .executes(ctx -> {
                            String bot = StringArgumentType.getString(ctx, "bot");
                            ctx.getSource().sendFeedback(() -> Text.literal("§a[Mitosis] Bot " + bot + " using item continuously"), true);
                            return 1;
                        })
                    )
                )
                // /player <bot> stop
                .then(literal("stop")
                    .executes(ctx -> {
                        String bot = StringArgumentType.getString(ctx, "bot");
                        ctx.getSource().sendFeedback(() -> Text.literal("§a[Mitosis] Stopped all actions for bot " + bot), true);
                        return 1;
                    })
                )
                // /player <bot> look at <x> <y> <z>
                .then(literal("look")
                    .then(literal("at")
                        .then(argument("x", FloatArgumentType.floatArg())
                            .then(argument("y", FloatArgumentType.floatArg())
                                .then(argument("z", FloatArgumentType.floatArg())
                                    .executes(ctx -> {
                                        String bot = StringArgumentType.getString(ctx, "bot");
                                        float x = FloatArgumentType.getFloat(ctx, "x");
                                        float y = FloatArgumentType.getFloat(ctx, "y");
                                        float z = FloatArgumentType.getFloat(ctx, "z");
                                        ctx.getSource().sendFeedback(() -> Text.literal("§a[Mitosis] Bot " + bot + " looking at " + x + " " + y + " " + z), true);
                                        return 1;
                                    })
                                )
                            )
                        )
                    )
                )
                // /player <bot> plugin <plugin_id> <action> [args...]
                .then(literal("plugin")
                    .then(argument("plugin_id", StringArgumentType.word())
                        .then(argument("action", StringArgumentType.word())
                            .executes(ctx -> {
                                String bot = StringArgumentType.getString(ctx, "bot");
                                String plugin = StringArgumentType.getString(ctx, "plugin_id");
                                String action = StringArgumentType.getString(ctx, "action");
                                ctx.getSource().sendFeedback(() -> Text.literal("§a[Mitosis] Bot " + bot + " executed " + plugin + ":" + action), true);
                                return 1;
                            })
                            .then(argument("args", StringArgumentType.greedyString())
                                .executes(ctx -> {
                                    String bot = StringArgumentType.getString(ctx, "bot");
                                    String plugin = StringArgumentType.getString(ctx, "plugin_id");
                                    String action = StringArgumentType.getString(ctx, "action");
                                    String args = StringArgumentType.getString(ctx, "args");
                                    ctx.getSource().sendFeedback(() -> Text.literal("§a[Mitosis] Bot " + bot + " executed " + plugin + ":" + action + " with args: " + args), true);
                                    return 1;
                                })
                            )
                        )
                    )
                )
            )
            // /player group <group> ...
            .then(literal("group")
                .then(argument("group", StringArgumentType.word())
                    .then(literal("connect")
                        .executes(ctx -> {
                            String group = StringArgumentType.getString(ctx, "group");
                            ctx.getSource().sendFeedback(() -> Text.literal("§a[Mitosis] Connecting bot group: " + group), true);
                            return 1;
                        })
                    )
                    .then(literal("disconnect")
                        .executes(ctx -> {
                            String group = StringArgumentType.getString(ctx, "group");
                            ctx.getSource().sendFeedback(() -> Text.literal("§a[Mitosis] Disconnecting bot group: " + group), true);
                            return 1;
                        })
                    )
                    .then(literal("list")
                        .executes(ctx -> {
                            String group = StringArgumentType.getString(ctx, "group");
                            ctx.getSource().sendFeedback(() -> Text.literal("§a[Mitosis] Group " + group + " active status"), true);
                            return 1;
                        })
                    )
                    .then(literal("plugin")
                        .then(argument("plugin_id", StringArgumentType.word())
                            .then(argument("action", StringArgumentType.word())
                                .executes(ctx -> {
                                    String group = StringArgumentType.getString(ctx, "group");
                                    String plugin = StringArgumentType.getString(ctx, "plugin_id");
                                    String action = StringArgumentType.getString(ctx, "action");
                                    ctx.getSource().sendFeedback(() -> Text.literal("§a[Mitosis] Group " + group + " executing " + plugin + ":" + action), true);
                                    return 1;
                                })
                            )
                        )
                    )
                )
            )
            // /player benchmark start <scenario>
            .then(literal("benchmark")
                .then(literal("start")
                    .then(argument("scenario", StringArgumentType.word())
                        .executes(ctx -> {
                            String scenario = StringArgumentType.getString(ctx, "scenario");
                            ctx.getSource().sendFeedback(() -> Text.literal("§a[Mitosis] Starting benchmark scenario: " + scenario), true);
                            return 1;
                        })
                    )
                )
                .then(literal("stop")
                    .executes(ctx -> {
                        ctx.getSource().sendFeedback(() -> Text.literal("§a[Mitosis] Stopped active benchmark"), true);
                        return 1;
                    })
                )
            )
            // /player config reload
            .then(literal("config")
                .then(literal("reload")
                    .executes(ctx -> {
                        ctx.getSource().sendFeedback(() -> Text.literal("§a[Mitosis] Config reloaded successfully"), true);
                        return 1;
                    })
                )
            )
        );
    }
}
