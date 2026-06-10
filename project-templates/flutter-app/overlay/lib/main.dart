import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:{{project_name}}/app_state.dart';
import 'package:{{project_name}}/screens/shell_screen.dart';
import 'package:{{project_name}}/screens/welcome_screen.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final prefs = await SharedPreferences.getInstance();
  final state = AppState(prefs);
  await state.load();
  runApp(AppRoot(state: state));
}

class AppRoot extends StatelessWidget {
  const AppRoot({super.key, required this.state});

  final AppState state;

  @override
  Widget build(BuildContext context) {
    return AppStateScope(
      state: state,
      child: AnimatedBuilder(
        animation: state,
        builder: (context, _) {
          return MaterialApp(
            title: '{{app_title}}',
            debugShowCheckedModeBanner: false,
            locale: const Locale('zh', 'CN'),
            supportedLocales: const [Locale('zh', 'CN')],
            localizationsDelegates: const [
              GlobalMaterialLocalizations.delegate,
              GlobalWidgetsLocalizations.delegate,
              GlobalCupertinoLocalizations.delegate,
            ],
            themeMode: state.themeMode,
            theme: _theme(Brightness.light),
            darkTheme: _theme(Brightness.dark),
            home: state.seenWelcome ? const ShellScreen() : const WelcomeScreen(),
          );
        },
      ),
    );
  }

  ThemeData _theme(Brightness brightness) {
    return ThemeData(
      useMaterial3: true,
      brightness: brightness,
      colorScheme: ColorScheme.fromSeed(
        seedColor: const Color(0xFF6B4EFF),
        brightness: brightness,
      ),
    );
  }
}

class AppStateScope extends InheritedWidget {
  const AppStateScope({super.key, required this.state, required super.child});

  final AppState state;

  static AppState of(BuildContext context) {
    final scope = context.dependOnInheritedWidgetOfExactType<AppStateScope>();
    assert(scope != null);
    return scope!.state;
  }

  @override
  bool updateShouldNotify(AppStateScope oldWidget) => state != oldWidget.state;
}
