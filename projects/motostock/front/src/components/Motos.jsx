import { useState, useEffect } from 'react'
import { Plus, Trash2, ClipboardList } from 'lucide-react'
import { api } from '../api'
import Modal from './Modal'

export default function Motos() {
  const [motos, setMotos] = useState([])
  const [showModal, setShowModal] = useState(false)
  const [form, setForm] = useState({ plate: '', brand: '', model: '', year: '2024', owner_name: '', owner_phone: '' })

  useEffect(() => {
    loadMotos()
  }, [])

  async function loadMotos() {
    try {
      const data = await api.getMotos()
      setMotos(Array.isArray(data) ? data : [])
    } catch (e) {
      console.error('Error loading motos:', e)
    }
  }

  async function handleCreate(e) {
    e.preventDefault()
    await api.createMoto(form)
    setShowModal(false)
    setForm({ plate: '', brand: '', model: '', year: '2024', owner_name: '', owner_phone: '' })
    loadMotos()
  }

  async function handleDelete(id) {
    if (!confirm('¿Eliminar moto?')) return
    await api.deleteMoto(id)
    loadMotos()
  }

  async function viewOrders(id) {
    try {
      const orders = await api.getMotoOrders(id)
      if (Array.isArray(orders) && orders.length) {
        alert('Historial de órdenes:\n\n' + orders.map(o => `#${o.id}: ${o.description} (${formatStatus(o.status)})`).join('\n'))
      } else {
        alert('Sin órdenes para esta moto')
      }
    } catch (e) {
      console.error('Error:', e)
    }
  }

  function formatStatus(status) {
    const map = { 'pending': 'Pendiente', 'in_progress': 'En Progreso', 'completed': 'Completada' }
    return map[status] || status
  }

  return (
    <section>
      <div className="section-header">
        <h2>Motos Registradas</h2>
        <button className="btn btn-primary" onClick={() => setShowModal(true)}>
          <Plus size={18} style={{ marginRight: '0.25rem', verticalAlign: 'middle' }} />
          Nueva Moto
        </button>
      </div>

      <div className="table-container">
        <table>
          <thead>
            <tr>
              <th>Patente</th>
              <th>Vehículo</th>
              <th>Año</th>
              <th>Propietario</th>
              <th>Teléfono</th>
              <th>Acciones</th>
            </tr>
          </thead>
          <tbody>
            {motos.length > 0 ? motos.map(m => (
              <tr key={m.id}>
                <td><strong>{m.plate}</strong></td>
                <td>{m.brand} {m.model}</td>
                <td>{m.year || '-'}</td>
                <td>{m.owner_name}</td>
                <td>{m.owner_phone || '-'}</td>
                <td className="actions">
                  <button className="btn-sm" onClick={() => viewOrders(m.id)} title="Ver órdenes">
                    <ClipboardList size={14} />
                  </button>
                  <button className="btn-sm btn-danger" onClick={() => handleDelete(m.id)} title="Eliminar">
                    <Trash2 size={14} />
                  </button>
                </td>
              </tr>
            )) : (
              <tr><td colSpan="6" className="empty-state">No hay motos registradas</td></tr>
            )}
          </tbody>
        </table>
      </div>

      {showModal && (
        <Modal title="Nueva Moto" onClose={() => setShowModal(false)}>
          <form onSubmit={handleCreate}>
            <div className="modal-body">
              <input placeholder="Patente (ej: AB123CD)" value={form.plate} onChange={e => setForm({...form, plate: e.target.value})} required />
              <input placeholder="Marca (ej: Honda)" value={form.brand} onChange={e => setForm({...form, brand: e.target.value})} required />
              <input placeholder="Modelo (ej: CG 150)" value={form.model} onChange={e => setForm({...form, model: e.target.value})} required />
              <input type="number" placeholder="Año" value={form.year} onChange={e => setForm({...form, year: e.target.value})} />
              <input placeholder="Nombre del propietario" value={form.owner_name} onChange={e => setForm({...form, owner_name: e.target.value})} required />
              <input placeholder="Teléfono" value={form.owner_phone} onChange={e => setForm({...form, owner_phone: e.target.value})} />
            </div>
            <div className="modal-footer">
              <button type="button" className="btn" onClick={() => setShowModal(false)}>Cancelar</button>
              <button type="submit" className="btn btn-primary">Registrar Moto</button>
            </div>
          </form>
        </Modal>
      )}
    </section>
  )
}
