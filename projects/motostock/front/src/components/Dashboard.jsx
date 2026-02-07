import { useState, useEffect } from 'react'
import { Package, AlertTriangle, Wrench, Bike, ClipboardList, ChevronRight, AlertCircle } from 'lucide-react'
import { api } from '../api'

export default function Dashboard({ onNavigate }) {
  const [stats, setStats] = useState({ totalValue: 0, totalUnits: 0, partsCount: 0, lowStockCount: 0, pendingOrders: 0, inProgressOrders: 0 })
  const [recentOrders, setRecentOrders] = useState([])
  const [lowStock, setLowStock] = useState([])

  useEffect(() => {
    loadData()
  }, [])

  async function loadData() {
    try {
      const [parts, lowStockData, orders] = await Promise.all([
        api.getParts(),
        api.getLowStock(),
        api.getOrders()
      ])

      const partsArr = Array.isArray(parts) ? parts : []
      const ordersArr = Array.isArray(orders) ? orders : []
      const lowArr = Array.isArray(lowStockData) ? lowStockData : []

      setStats({
        totalValue: partsArr.reduce((sum, p) => sum + (p.price * p.stock), 0),
        totalUnits: partsArr.reduce((sum, p) => sum + p.stock, 0),
        partsCount: partsArr.length,
        lowStockCount: lowArr.length,
        pendingOrders: ordersArr.filter(o => o.status === 'pending').length,
        inProgressOrders: ordersArr.filter(o => o.status === 'in_progress').length
      })

      setRecentOrders(ordersArr.slice(0, 5))
      setLowStock(lowArr)
    } catch (e) {
      console.error('Error loading dashboard:', e)
    }
  }

  function getDaysAgo(dateStr) {
    const date = new Date(dateStr)
    const days = Math.floor((Date.now() - date) / (1000 * 60 * 60 * 24))
    if (days === 0) return 'Hoy'
    if (days === 1) return 'Ayer'
    return `Hace ${days} días`
  }

  return (
    <section>
      <div className="section-header">
        <h2>Dashboard</h2>
      </div>

      <div className="stats-grid">
        <div className="stat-card">
          <div className="stat-icon blue"><Package size={24} /></div>
          <div className="stat-content">
            <div className="stat-value">${stats.totalValue.toFixed(2)}</div>
            <div className="stat-label">{stats.partsCount} productos, {stats.totalUnits} unidades</div>
          </div>
        </div>
        <div className="stat-card">
          <div className={`stat-icon ${stats.lowStockCount > 0 ? 'red' : 'green'}`}><AlertTriangle size={24} /></div>
          <div className="stat-content">
            <div className={`stat-value ${stats.lowStockCount > 0 ? 'low-stock' : ''}`}>{stats.lowStockCount}</div>
            <div className="stat-label">productos bajo mínimo</div>
          </div>
        </div>
        <div className="stat-card">
          <div className="stat-icon yellow"><Wrench size={24} /></div>
          <div className="stat-content">
            <div className="stat-value">{stats.pendingOrders}</div>
            <div className="stat-label">órdenes pendientes</div>
          </div>
        </div>
        <div className="stat-card">
          <div className="stat-icon blue"><Bike size={24} /></div>
          <div className="stat-content">
            <div className="stat-value">{stats.inProgressOrders}</div>
            <div className="stat-label">en progreso</div>
          </div>
        </div>
      </div>

      <div className="dashboard-grid">
        <div className="table-container">
          <div className="table-header">
            <span style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
              <ClipboardList size={18} /> Órdenes Recientes
            </span>
            <span className="link" onClick={() => onNavigate('orders')} style={{ display: 'flex', alignItems: 'center', gap: '0.25rem' }}>
              Ver todas <ChevronRight size={16} />
            </span>
          </div>
          <div>
            {recentOrders.length > 0 ? (
              recentOrders.map(o => (
                <div className="order-item" key={o.id}>
                  <span className={`badge badge-${o.status}`}>{o.status}</span>
                  <div className="info">
                    <div className="title">{o.plate} - {o.brand} {o.model}</div>
                    <div className="subtitle">{o.description || 'Sin descripción'}</div>
                  </div>
                  <div className="date">{getDaysAgo(o.created_at)}</div>
                </div>
              ))
            ) : (
              <div className="empty-state">No hay órdenes recientes</div>
            )}
          </div>
        </div>

        <div className="table-container">
          <div className="table-header">
            <span style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
              <AlertTriangle size={18} /> Alertas de Stock
            </span>
          </div>
          <div>
            {lowStock.length > 0 ? (
              lowStock.map(p => (
                <div className="alert-item" key={p.id}>
                  <div className="icon"><AlertCircle size={20} color="#ef4444" /></div>
                  <div className="text">
                    <div className="title">{p.name}</div>
                    <div className="subtitle">Stock: {p.stock} / Mín: {p.min_stock}</div>
                  </div>
                </div>
              ))
            ) : (
              <div className="empty-state" style={{ padding: '1rem' }}>Sin alertas de stock</div>
            )}
          </div>
        </div>
      </div>
    </section>
  )
}
