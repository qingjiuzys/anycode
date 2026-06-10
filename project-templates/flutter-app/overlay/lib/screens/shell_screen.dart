import 'package:flutter/material.dart';
import 'package:{{project_name}}/main.dart';

class ShellScreen extends StatelessWidget {
  const ShellScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('{{app_title}}')),
      body: ListView(
        padding: const EdgeInsets.all(20),
        children: [
          Text(
            '模板已就绪',
            style: Theme.of(context).textTheme.headlineSmall,
          ),
          const SizedBox(height: 8),
          const Text(
            '在 anycode 中描述你的产品需求，Agent 会更新本项目的页面、状态与测试。',
          ),
          const SizedBox(height: 16),
          const Text('建议流程：'),
          const Text('1. 完善 PRODUCT_BRIEF.md'),
          const Text('2. 实现 MVP 页面'),
          const Text('3. Agent 跑 flutter analyze/test 或 Dashboard 门禁'),
        ],
      ),
      bottomNavigationBar: NavigationBar(
        destinations: const [
          NavigationDestination(icon: Icon(Icons.home_outlined), label: '首页'),
          NavigationDestination(icon: Icon(Icons.person_outline), label: '我的'),
        ],
        selectedIndex: 0,
        onDestinationSelected: (_) {},
      ),
    );
  }
}
