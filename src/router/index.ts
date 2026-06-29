import { createRouter, createWebHistory } from 'vue-router';
import { useAuthStore } from '@/stores/auth';

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      redirect: '/login'
    },
    {
      path: '/login',
      name: 'Login',
      component: () => import('../views/Login.vue')
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
      path: '/profile',
      name: 'Profile',
      component: () => import('../views/Profile.vue')
    },
    {
      path: '/test',
      name: 'Test',
      component: () => import('../views/Test.vue')
    }
  ]
});

router.beforeEach((to) => {
  const auth = useAuthStore();

  if (to.path !== '/login' && !auth.isLoggedIn) {
    auth.setRedirectPath(to.path);
    return '/login';
  } else if (to.path === '/login' && auth.isLoggedIn) {
    return '/dashboard';
  }
  return true;
});

export default router;