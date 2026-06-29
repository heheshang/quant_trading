import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import StrategyParamEditor from '@/components/strategy/StrategyParamEditor.vue'
import type { ParameterSchema } from '@/services/types'

// Mock Element Plus components (auto-import resolvers are disabled in test mode)
const mockComponents = {
  'el-form-item': {
    template: '<div class="el-form-item"><label v-if="label">{{ label }}</label><slot /></div>',
    props: ['label', 'prop', 'required'],
  },
  'el-input-number': {
    template:
      '<input class="el-input-number" type="number" :value="modelValue" :min="min" :max="max" :step="step" :precision="precision" @input="onInput" />',
    props: ['modelValue', 'min', 'max', 'step', 'precision'],
    methods: {
      onInput(this: { $emit: (event: string, ...args: unknown[]) => void }, e: Event) {
        const val = Number((e.target as HTMLInputElement).value)
        this.$emit('update:modelValue', val)
        this.$emit('change')
      },
    },
  },
  'el-input': {
    template:
      '<input class="el-input" :value="modelValue" @input="onInput" />',
    props: ['modelValue', 'placeholder'],
    methods: {
      onInput(this: { $emit: (event: string, ...args: unknown[]) => void }, e: Event) {
        this.$emit('update:modelValue', (e.target as HTMLInputElement).value)
      },
    },
  },
  'el-select': {
    template:
      '<select class="el-select" :value="modelValue" @change="onChange"><slot /></select>',
    props: ['modelValue', 'placeholder'],
    methods: {
      onChange(this: { $emit: (event: string, ...args: unknown[]) => void }, e: Event) {
        this.$emit('update:modelValue', (e.target as HTMLSelectElement).value)
        this.$emit('change')
      },
    },
  },
  'el-option': {
    template: '<option class="el-option" :value="value">{{ label }}</option>',
    props: ['value', 'label'],
  },
}

// ── Fixtures ──

const numberSchema: ParameterSchema = {
  name: 'lookback',
  param_type: 'Number',
  default: 20,
  range: { min: 1, max: 100, step: 1 },
  description: 'Lookback period',
}

const selectSchema: ParameterSchema = {
  name: 'method',
  param_type: { Select: ['sma', 'ema', 'wma'] },
  default: 'sma',
  description: 'Moving average method',
}

const stringSchema: ParameterSchema = {
  name: 'label',
  param_type: 'String',
  default: 'custom-label',
  description: 'Custom label',
}

describe('StrategyParamEditor', () => {
  const defaults = { global: { components: mockComponents } }

  // ── Schema rendering ──

  it('renders all schema parameters as form items', () => {
    const schema = [numberSchema, selectSchema, stringSchema]
    const wrapper = mount(StrategyParamEditor, {
      props: { schema, modelValue: {} },
      ...defaults,
    })
    expect(wrapper.text()).toContain('Lookback period')
    expect(wrapper.text()).toContain('Moving average method')
    expect(wrapper.text()).toContain('Custom label')
    expect(wrapper.findAll('.el-form-item')).toHaveLength(3)
  })

  it('renders correct input types per param_type', () => {
    const schema = [numberSchema, selectSchema, stringSchema]
    const wrapper = mount(StrategyParamEditor, {
      props: { schema, modelValue: {} },
      ...defaults,
    })
    expect(wrapper.find('.el-input-number').exists()).toBe(true)
    expect(wrapper.find('.el-select').exists()).toBe(true)
    expect(wrapper.find('.el-input').exists()).toBe(true)
  })

  it('empty schema renders nothing', () => {
    const wrapper = mount(StrategyParamEditor, {
      props: { schema: [], modelValue: {} },
      ...defaults,
    })
    expect(wrapper.find('.el-form-item').exists()).toBe(false)
    expect(wrapper.text()).toBe('')
  })

  // ── Default values ──

  it('initializes localValue with schema defaults when modelValue is empty', async () => {
    const schema = [numberSchema, selectSchema]
    const wrapper = mount(StrategyParamEditor, {
      props: { schema, modelValue: {} },
      ...defaults,
    })
    // Trigger a change to emit the full localValue
    const numberInput = wrapper.find('.el-input-number')
    const el = numberInput.element as HTMLInputElement
    el.value = '20'
    await numberInput.trigger('input')
    const emitted = wrapper.emitted('update:modelValue')
    expect(emitted).toBeTruthy()
    expect(emitted![0][0]).toEqual({ lookback: 20, method: 'sma' })
  })

  it('buildDefaults fills missing keys from schema', async () => {
    // modelValue only has lookback, method should get default
    const schema = [numberSchema, selectSchema]
    const wrapper = mount(StrategyParamEditor, {
      props: { schema, modelValue: { lookback: 15 } },
      ...defaults,
    })
    const numberInput = wrapper.find('.el-input-number')
    const el = numberInput.element as HTMLInputElement
    el.value = '15'
    await numberInput.trigger('input')
    const emitted = wrapper.emitted('update:modelValue')
    // lookback=15 from prop, method='sma' from schema default
    expect(emitted![0][0]).toEqual({ lookback: 15, method: 'sma' })
  })

  it('buildDefaults preserves existing modelValue values', async () => {
    const schema = [numberSchema, selectSchema, stringSchema]
    const wrapper = mount(StrategyParamEditor, {
      props: { schema, modelValue: { lookback: 15, label: 'my-label' } },
      ...defaults,
    })
    const numberInput = wrapper.find('.el-input-number')
    const el = numberInput.element as HTMLInputElement
    el.value = '15'
    await numberInput.trigger('input')
    const emitted = wrapper.emitted('update:modelValue')
    // lookback=15 from prop, method='sma' from default, label='my-label' from prop
    expect(emitted![0][0]).toEqual({ lookback: 15, method: 'sma', label: 'my-label' })
  })

  it('skips null and undefined defaults', async () => {
    const schema: ParameterSchema[] = [
      { name: 'color', param_type: 'String', default: null, description: 'Color' },
      { name: 'size', param_type: 'Number', default: 10, description: 'Size' },
    ]
    const wrapper = mount(StrategyParamEditor, {
      props: { schema, modelValue: {} },
      ...defaults,
    })
    const numberInput = wrapper.find('.el-input-number')
    const el = numberInput.element as HTMLInputElement
    el.value = '10'
    await numberInput.trigger('input')
    const emitted = wrapper.emitted('update:modelValue')
    expect(emitted![0][0]).toEqual({ size: 10 })
    expect((emitted![0][0] as Record<string, unknown>).color).toBeUndefined()
  })

  // ── modelValue emit ──

  it('modelValue changes propagate via emit on number input', async () => {
    const schema = [numberSchema, selectSchema]
    const wrapper = mount(StrategyParamEditor, {
      props: { schema, modelValue: { lookback: 50, method: 'sma' } },
      ...defaults,
    })
    const numberInput = wrapper.find('.el-input-number')
    const el = numberInput.element as HTMLInputElement
    el.value = '30'
    await numberInput.trigger('input')
    const emitted = wrapper.emitted('update:modelValue')
    expect(emitted![0][0]).toEqual({ lookback: 30, method: 'sma' })
  })

  it('modelValue changes propagate via emit on select change', async () => {
    const schema = [numberSchema, selectSchema]
    const wrapper = mount(StrategyParamEditor, {
      props: { schema, modelValue: { lookback: 20, method: 'sma' } },
      ...defaults,
    })
    const select = wrapper.find('.el-select')
    const el = select.element as HTMLSelectElement
    el.value = 'ema'
    await select.trigger('change')
    const emitted = wrapper.emitted('update:modelValue')
    expect(emitted![0][0]).toEqual({ lookback: 20, method: 'ema' })
  })

  it('modelValue changes propagate via emit on string input', async () => {
    const schema = [stringSchema]
    const wrapper = mount(StrategyParamEditor, {
      props: { schema, modelValue: { label: 'old' } },
      ...defaults,
    })
    const strInput = wrapper.find('.el-input')
    const el = strInput.element as HTMLInputElement
    el.value = 'new-label'
    await strInput.trigger('input')
    const emitted = wrapper.emitted('update:modelValue')
    expect(emitted![0][0]).toEqual({ label: 'new-label' })
  })

  // ── Watcher ──

  it('watch on modelValue merges defaults when prop changes', async () => {
    const schema = [numberSchema, selectSchema]
    const wrapper = mount(StrategyParamEditor, {
      props: { schema, modelValue: { lookback: 10 } },
      ...defaults,
    })
    // modelValue prop changes — watch should merge
    await wrapper.setProps({ modelValue: { lookback: 99 } })
    // Trigger change to emit current localValue
    const numberInput = wrapper.find('.el-input-number')
    const el = numberInput.element as HTMLInputElement
    el.value = '99'
    await numberInput.trigger('input')
    const emitted = wrapper.emitted('update:modelValue')
    // After watch merges: lookback=99, method='sma' (default filled)
    expect(emitted![0][0]).toEqual({ lookback: 99, method: 'sma' })
  })

  it('watch on modelValue preserves existing values on prop update', async () => {
    const schema = [numberSchema, selectSchema]
    const wrapper = mount(StrategyParamEditor, {
      props: { schema, modelValue: { lookback: 10, method: 'ema' } },
      ...defaults,
    })
    await wrapper.setProps({ modelValue: { lookback: 25, method: 'wma' } })
    const numberInput = wrapper.find('.el-input-number')
    const el = numberInput.element as HTMLInputElement
    el.value = '25'
    await numberInput.trigger('input')
    const emitted = wrapper.emitted('update:modelValue')
    expect(emitted![0][0]).toEqual({ lookback: 25, method: 'wma' })
  })

  // ── Select option rendering ──

  it('renders select options from schema', () => {
    const schema = [selectSchema]
    const wrapper = mount(StrategyParamEditor, {
      props: { schema, modelValue: {} },
      ...defaults,
    })
    const options = wrapper.findAll('.el-option')
    expect(options).toHaveLength(3)
    expect(options[0].text()).toBe('sma')
    expect(options[1].text()).toBe('ema')
    expect(options[2].text()).toBe('wma')
  })

  // ── Range props ──

  it('passes range constraints to number input', () => {
    const schema = [numberSchema]
    const wrapper = mount(StrategyParamEditor, {
      props: { schema, modelValue: {} },
      ...defaults,
    })
    const numberInput = wrapper.find('.el-input-number')
    // The mock el-input-number receives min/max/step as props
    // Vue Test Utils exposes props via wrapper's props()
    // but since it's a mock component, we can check attributes
    expect(numberInput.attributes('min')).toBe('1')
    expect(numberInput.attributes('max')).toBe('100')
    expect(numberInput.attributes('step')).toBe('1')
  })

  it('handles missing range gracefully', () => {
    const schema: ParameterSchema[] = [
      { name: 'threshold', param_type: 'Number', default: 0.5, description: 'Threshold' },
    ]
    const wrapper = mount(StrategyParamEditor, {
      props: { schema, modelValue: {} },
      ...defaults,
    })
    const numberInput = wrapper.find('.el-input-number')
    // Without range props, min/max/step are not set as attributes
    // (the component uses ?? with -Infinity/Infinity which don't render as HTML attrs)
    expect(numberInput.exists()).toBe(true)
  })
})
