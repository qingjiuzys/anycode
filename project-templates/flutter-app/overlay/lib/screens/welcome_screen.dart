import 'package:flutter/material.dart';
import 'package:{{project_name}}/main.dart';
import 'package:{{project_name}}/screens/shell_screen.dart';

class WelcomeScreen extends StatelessWidget {
  const WelcomeScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.all(28),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Spacer(),
              Text(
                '{{app_title}}',
                style: Theme.of(context).textTheme.displaySmall?.copyWith(
                      fontWeight: FontWeight.bold,
                    ),
              ),
              const SizedBox(height: 12),
              const Text('使用 anycode 对话在此项目上生成完整 Flutter 应用。'),
              const Spacer(),
              SizedBox(
                width: double.infinity,
                child: FilledButton(
                  key: const Key('onboarding_start'),
                  onPressed: () async {
                    await AppStateScope.of(context).completeWelcome();
                    if (!context.mounted) return;
                    Navigator.of(context).pushReplacement(
                      MaterialPageRoute<void>(builder: (_) => const ShellScreen()),
                    );
                  },
                  child: const Padding(
                    padding: EdgeInsets.symmetric(vertical: 14),
                    child: Text('开始探索'),
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
