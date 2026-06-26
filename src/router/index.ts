import { createRouter, createWebHistory, RouteLocationNormalized } from 'vue-router';

// Authentication guard
const requireAuth = (to: RouteLocationNormalized, _from: RouteLocationNormalized, next: any) => {
  const isAuthenticated = localStorage.getItem('isAuthenticated');
  
  // Only allow access to login page when not authenticated
  if (to.path !== '/login' && !isAuthenticated) {
    // Store intended redirect path for post-login
    localStorage.setItem('redirect_after_login', to.path);
    next('/login');
  } else if (to.path === '/login' && isAuthenticated) {
    // If already authenticated, redirect to dashboard
    next('/dashboard');
  } else {
    // Allow access to requested route
    next();
  }
};

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
      component: () => import('../views/Dashboard.vue'),
      beforeEnter: requireAuth
    },
    {
      path: '/strategy',
      name: 'Strategy',
      component: () => import('../views/Strategy.vue'),
      beforeEnter: requireAuth
    },
    {
      path: '/backtest',
      name: 'Backtest',
      component: () => import('../views/Backtest.vue'),
      beforeEnter: requireAuth
    },
    {
      path: '/trading',
      name: 'Trading',
      component: () => import('../views/Trading.vue'),
      beforeEnter: requireAuth
    },
    {
      path: '/risk',
      name: 'Risk',
      component: () => import('../views/Risk.vue'),
      beforeEnter: requireAuth
    },
    {
      path: '/monitor',
      name: 'Monitor',
      component: () => import('../views/Monitor.vue'),
      beforeEnter: requireAuth
    },
    {
      path: '/settings',
      name: 'Settings',
      component: () => import('../views/Settings.vue'),
      beforeEnter: requireAuth
    },
    {
      path: '/profile',
      name: 'Profile',
      component: () => import('../views/Profile.vue'),
      beforeEnter: requireAuth
    },
    {
      path: '/test',
      name: 'Test',
      component: () => import('../views/Test.vue'),
      beforeEnter: requireAuth
    }
  ]
});

// Global authentication guard
router.beforeEach(requireAuth);

export default router;