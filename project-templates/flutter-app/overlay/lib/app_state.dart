import 'package:flutter/material.dart';
import 'package:shared_preferences/shared_preferences.dart';

class AppState extends ChangeNotifier {
  AppState(this._prefs);

  final SharedPreferences _prefs;
  static const _welcomeKey = 'seen_welcome';
  static const _themeKey = 'theme_mode';

  bool seenWelcome = false;
  ThemeMode themeMode = ThemeMode.system;

  Future<void> load() async {
    seenWelcome = _prefs.getBool(_welcomeKey) ?? false;
    final theme = _prefs.getString(_themeKey);
    if (theme == 'light') themeMode = ThemeMode.light;
    if (theme == 'dark') themeMode = ThemeMode.dark;
    notifyListeners();
  }

  Future<void> completeWelcome() async {
    seenWelcome = true;
    await _prefs.setBool(_welcomeKey, true);
    notifyListeners();
  }

  Future<void> setThemeMode(ThemeMode mode) async {
    themeMode = mode;
    final v = switch (mode) {
      ThemeMode.light => 'light',
      ThemeMode.dark => 'dark',
      ThemeMode.system => 'system',
    };
    await _prefs.setString(_themeKey, v);
    notifyListeners();
  }
}
