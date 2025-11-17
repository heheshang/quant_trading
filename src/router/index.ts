import { createRouter, createWebHistory } from 'vue-router';

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      redirect: '/dashboard'
    },
    {
      path: '/dashboard',
      name: 'Dashboard',
      component: () => import('../views/Dashboard.vue')
    },
    {
      path: '/strategy',
      name: 'Strategy',
      component: () => import('../views/Strategy.vue')
    },
    {
      path: '/backtest',
      name: 'Backtest',
      component: () => import('../views/Backtest.vue')
    },
    {
      path: '/trading',
      name: 'Trading',
      component: () => import('../views/Trading.vue')
    },
    {
      path: '/risk',
      name: 'Risk',
      component: () => import('../views/Risk.vue')
    },
    {
      path: '/monitor',
      name: 'Monitor',
      component: () => import('../views/Monitor.vue')
    },
    {
      path: '/settings',
      name: 'Settings',
      component: () => import('../views/Settings.vue')
    },
    {
      path: '/test',
      name: 'Test',
      component: () => import('../views/Test.vue')
    }
  ]
});

export default router;