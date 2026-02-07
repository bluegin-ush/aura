import { useState, useEffect } from 'react'
import { Plus, Trash2, Search } from 'lucide-react'
import { api } from '../api'
import Modal from './Modal'

export default function Parts() {
  const [parts, setParts] = useState([])
  const [search, setSearch] = useState('')
  const [showModal, setShowModal] = useState(false)
  const [form, setForm] = useState({ code: '', name: '', brand: '', price: '', stock: '0', min_stock: '5' })

  useEffect(() => {
    loadParts()
  }, [])

  async function loadParts() {
    try {
      const data = await api.getParts()
      setParts(Array.isArray(data) ? data : [])
    } catch (e) {
      console.error('Error loading parts:', e)
    }
  }

  async function handleSearch(value) {
    setSearch(value)
    try {
      const data = value ? await api.searchParts(value) : await api.getParts()
      setParts(Array.isArray(data) ? data : [])
    } catch (e) {
      console.error('Error searching:', e)
    }
  }

  async function handleCreate(e) {
    e.preventDefault()
    await api.createPart(form)
    setShowModal(false)
    setForm({ code: '', name: '', brand: '', price: '', stock: '0', min_stock: '5' })
    loadParts()
  }

  async function handleDelete(id) {
    if (!confirm('¿Eliminar repuesto?')) return
    await api.deletePart(id)
    loadParts()
  }

  return (
    <section>
      <div className="section-header">
        <h2>Repuestos</h2>
        <button className="btn btn-primary" onClick={() => setShowModal(true)}>
          <Plus size={18} style={{ marginRight: '0.25rem', verticalAlign: 'middle' }} />
          Nuevo Repuesto
        </button>
      </div>

      <div className="table-container">
        <div className="search-box" style={{ position: 'relative' }}>
          <Search size={18} style={{ position: 'absolute', left: '1.75rem', top: '50%', transform: 'translateY(-50%)', color: '#6b7280' }} />
          <input
            type="text"
            placeholder="Buscar por código, nombre o marca..."
            value={search}
            onChange={(e) => handleSearch(e.target.value)}
            style={{ paddingLeft: '2.5rem' }}
          />
        </div>
        <table>
          <thead>
            <tr>
              <th>Código</th>
              <th>Nombre</th>
              <th>Marca</th>
              <th>Precio</th>
              <th>Stock</th>
              <th>Acciones</th>
            </tr>
          </thead>
          <tbody>
            {parts.length > 0 ? parts.map(p => (
              <tr key={p.id}>
                <td><strong>{p.code}</strong></td>
                <td>{p.name}</td>
                <td>{p.brand || '-'}</td>
                <td>${(p.price || 0).toFixed(2)}</td>
                <td className={p.stock < p.min_stock ? 'low-stock' : ''}>{p.stock} / {p.min_stock}</td>
                <td className="actions">
                  <button className="btn-sm btn-danger" onClick={() => handleDelete(p.id)} title="Eliminar">
                    <Trash2 size={14} />
                  </button>
                </td>
              </tr>
            )) : (
              <tr><td colSpan="6" className="empty-state">No hay repuestos</td></tr>
            )}
          </tbody>
        </table>
      </div>

      {showModal && (
        <Modal title="Nuevo Repuesto" onClose={() => setShowModal(false)}>
          <form onSubmit={handleCreate}>
            <div className="modal-body">
              <input placeholder="Código (ej: ACE-001)" value={form.code} onChange={e => setForm({...form, code: e.target.value})} required />
              <input placeholder="Nombre del repuesto" value={form.name} onChange={e => setForm({...form, name: e.target.value})} required />
              <input placeholder="Marca" value={form.brand} onChange={e => setForm({...form, brand: e.target.value})} />
              <input type="number" step="0.01" placeholder="Precio" value={form.price} onChange={e => setForm({...form, price: e.target.value})} required />
              <input type="number" placeholder="Stock inicial" value={form.stock} onChange={e => setForm({...form, stock: e.target.value})} />
              <input type="number" placeholder="Stock mínimo (alerta)" value={form.min_stock} onChange={e => setForm({...form, min_stock: e.target.value})} />
            </div>
            <div className="modal-footer">
              <button type="button" className="btn" onClick={() => setShowModal(false)}>Cancelar</button>
              <button type="submit" className="btn btn-primary">Crear Repuesto</button>
            </div>
          </form>
        </Modal>
      )}
    </section>
  )
}
