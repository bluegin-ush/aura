import { useState, useEffect } from 'react'
import { Plus, Trash2, Package, Check, Clock, Wrench, CircleCheck, PlusCircle } from 'lucide-react'
import { api } from '../api'
import Modal from './Modal'

export default function Orders() {
  const [orders, setOrders] = useState([])
  const [motos, setMotos] = useState([])
  const [parts, setParts] = useState([])
  const [stats, setStats] = useState({ pending: 0, inProgress: 0, completed: 0 })
  const [showOrderModal, setShowOrderModal] = useState(false)
  const [showItemModal, setShowItemModal] = useState(false)
  const [selectedOrderId, setSelectedOrderId] = useState(null)
  const [orderForm, setOrderForm] = useState({ moto_id: '', description: '' })
  const [itemForm, setItemForm] = useState({ part_id: '', quantity: '1' })

  useEffect(() => {
    loadData()
  }, [])

  async function loadData() {
    try {
      const [ordersData, motosData, partsData] = await Promise.all([
        api.getOrders(),
        api.getMotos(),
        api.getParts()
      ])
      const ordersArr = Array.isArray(ordersData) ? ordersData : []
      setOrders(ordersArr)
      setMotos(Array.isArray(motosData) ? motosData : [])
      setParts(Array.isArray(partsData) ? partsData : [])
      setStats({
        pending: ordersArr.filter(o => o.status === 'pending').length,
        inProgress: ordersArr.filter(o => o.status === 'in_progress').length,
        completed: ordersArr.filter(o => o.status === 'completed').length
      })
    } catch (e) {
      console.error('Error loading orders:', e)
    }
  }

  async function handleCreateOrder(e) {
    e.preventDefault()
    await api.createOrder(orderForm.moto_id, orderForm.description)
    setShowOrderModal(false)
    setOrderForm({ moto_id: '', description: '' })
    loadData()
  }

  async function handleAddItem(e) {
    e.preventDefault()
    await api.addOrderItem(selectedOrderId, itemForm.part_id, itemForm.quantity)
    setShowItemModal(false)
    setItemForm({ part_id: '', quantity: '1' })
    loadData()
  }

  async function handleStatusChange(id) {
    const choice = prompt('Nuevo estado:\n1 = Pendiente\n2 = En Progreso\n3 = Completada\n\nIngrese número:')
    const statusMap = { '1': 'pending', '2': 'in_progress', '3': 'completed' }
    const newStatus = statusMap[choice]
    if (!newStatus) return
    await api.updateOrderStatus(id, newStatus)
    loadData()
  }

  async function handleDelete(id) {
    if (!confirm('¿Eliminar orden?')) return
    await api.deleteOrder(id)
    loadData()
  }

  async function viewItems(id) {
    try {
      const items = await api.getOrderItems(id)
      if (Array.isArray(items) && items.length) {
        const total = items.reduce((sum, i) => sum + (i.quantity * i.unit_price), 0)
        alert('Items de la orden #' + id + ':\n\n' +
          items.map(i => `${i.quantity}x ${i.name} - $${(i.unit_price * i.quantity).toFixed(2)}`).join('\n') +
          '\n\n─────────────\nTotal: $' + total.toFixed(2))
      } else {
        alert('Sin items en esta orden')
      }
    } catch (e) {
      console.error('Error:', e)
    }
  }

  function openItemModal(orderId) {
    setSelectedOrderId(orderId)
    setShowItemModal(true)
  }

  function formatStatus(status) {
    const map = { 'pending': 'Pendiente', 'in_progress': 'En Progreso', 'completed': 'Completada' }
    return map[status] || status
  }

  return (
    <section>
      <div className="section-header">
        <h2>Órdenes de Trabajo</h2>
        <button className="btn btn-primary" onClick={() => setShowOrderModal(true)}>
          <Plus size={18} style={{ marginRight: '0.25rem', verticalAlign: 'middle' }} />
          Nueva Orden
        </button>
      </div>

      <div className="stats-grid">
        <div className="stat-card">
          <div className="stat-icon yellow"><Clock size={24} /></div>
          <div className="stat-content">
            <div className="stat-value">{stats.pending}</div>
            <div className="stat-label">Pendientes</div>
          </div>
        </div>
        <div className="stat-card">
          <div className="stat-icon blue"><Wrench size={24} /></div>
          <div className="stat-content">
            <div className="stat-value">{stats.inProgress}</div>
            <div className="stat-label">En Progreso</div>
          </div>
        </div>
        <div className="stat-card">
          <div className="stat-icon green"><CircleCheck size={24} /></div>
          <div className="stat-content">
            <div className="stat-value">{stats.completed}</div>
            <div className="stat-label">Completadas</div>
          </div>
        </div>
      </div>

      <div className="table-container">
        <table>
          <thead>
            <tr>
              <th>Nº</th>
              <th>Moto</th>
              <th>Trabajo</th>
              <th>Estado</th>
              <th>Fecha</th>
              <th>Total</th>
              <th>Acciones</th>
            </tr>
          </thead>
          <tbody>
            {orders.length > 0 ? orders.map(o => (
              <tr key={o.id}>
                <td><strong>#{o.id}</strong></td>
                <td>{o.plate} - {o.brand} {o.model}</td>
                <td>{o.description || '-'}</td>
                <td><span className={`badge badge-${o.status}`}>{formatStatus(o.status)}</span></td>
                <td>{new Date(o.created_at).toLocaleDateString()}</td>
                <td>${(o.total || 0).toFixed(2)}</td>
                <td className="actions">
                  <button className="btn-sm" onClick={() => openItemModal(o.id)} title="Agregar repuesto">
                    <PlusCircle size={14} />
                  </button>
                  <button className="btn-sm" onClick={() => viewItems(o.id)} title="Ver items">
                    <Package size={14} />
                  </button>
                  <button className="btn-sm btn-success" onClick={() => handleStatusChange(o.id)} title="Cambiar estado">
                    <Check size={14} />
                  </button>
                  <button className="btn-sm btn-danger" onClick={() => handleDelete(o.id)} title="Eliminar">
                    <Trash2 size={14} />
                  </button>
                </td>
              </tr>
            )) : (
              <tr><td colSpan="7" className="empty-state">No hay órdenes</td></tr>
            )}
          </tbody>
        </table>
      </div>

      {showOrderModal && (
        <Modal title="Nueva Orden de Trabajo" onClose={() => setShowOrderModal(false)}>
          <form onSubmit={handleCreateOrder}>
            <div className="modal-body">
              <select value={orderForm.moto_id} onChange={e => setOrderForm({...orderForm, moto_id: e.target.value})} required>
                <option value="">Seleccionar moto...</option>
                {motos.map(m => <option key={m.id} value={m.id}>{m.plate} - {m.brand} {m.model}</option>)}
              </select>
              <input placeholder="Descripción del trabajo (ej: Service 10.000km)" value={orderForm.description} onChange={e => setOrderForm({...orderForm, description: e.target.value})} required />
            </div>
            <div className="modal-footer">
              <button type="button" className="btn" onClick={() => setShowOrderModal(false)}>Cancelar</button>
              <button type="submit" className="btn btn-primary">Crear Orden</button>
            </div>
          </form>
        </Modal>
      )}

      {showItemModal && (
        <Modal title="Agregar Repuesto a Orden" onClose={() => setShowItemModal(false)}>
          <form onSubmit={handleAddItem}>
            <div className="modal-body">
              <select value={itemForm.part_id} onChange={e => setItemForm({...itemForm, part_id: e.target.value})} required>
                <option value="">Seleccionar repuesto...</option>
                {parts.map(p => <option key={p.id} value={p.id}>{p.code} - {p.name} (${p.price}) [Stock: {p.stock}]</option>)}
              </select>
              <input type="number" min="1" placeholder="Cantidad" value={itemForm.quantity} onChange={e => setItemForm({...itemForm, quantity: e.target.value})} required />
            </div>
            <div className="modal-footer">
              <button type="button" className="btn" onClick={() => setShowItemModal(false)}>Cancelar</button>
              <button type="submit" className="btn btn-primary">Agregar</button>
            </div>
          </form>
        </Modal>
      )}
    </section>
  )
}
